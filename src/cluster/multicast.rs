use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, UdpSocket};
use std::time::Duration;

use actix_web::web;
use anyhow::bail;
use futures::executor::block_on;
use futures::FutureExt;
use lazy_static::lazy_static;
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::sync::broadcast;

use crate::server::AppState;

lazy_static! {
    static ref SEARCH: [u8; 4] = [0xAA, 0xBB, 0x01, 0x02];
}

fn bind_address(address: &SocketAddr, add: u16) -> SocketAddr {
    match &address {
        SocketAddr::V4(addr) => {
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, addr.port() + add))
        }
        SocketAddr::V6(addr) => SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::UNSPECIFIED,
            addr.port() + add,
            0,
            0,
        )),
    }
}

pub fn finder(
    address: &SocketAddr,
    timeout: Option<Duration>,
) -> anyhow::Result<Option<(SocketAddr, u16)>> {
    assert!(address.ip().is_multicast(), "address is not multicast");

    let bind_address = bind_address(&address, 1);
    let socket = UdpSocket::bind(bind_address)?;
    socket.set_read_timeout(timeout.or(Some(Duration::from_secs(3))))?;

    log::debug!("send multicast message to {}", address);
    match socket.send_to(&*SEARCH, address) {
        Ok(_) => log::debug!("send multicast message ok"),
        Err(e) => bail!(format!("send multicast message {}", e)),
    }

    let mut buf = [0u8; 4];
    return match socket.recv_from(&mut buf) {
        Ok((len, remote)) => {
            if len != 4 {
                return Ok(None);
            }
            log::debug!("multicast got data: {:?} from: {}", &buf, &remote);

            let n = u32::from_be_bytes(buf);
            let remote_port = ((n >> 16) & 0xFFFF) as u16;
            let remove_id = (n & 0xff) as u16;

            Ok(Some((SocketAddr::new(remote.ip(), remote_port), remove_id)))
        }
        #[cfg(unix)]
        // timeout in linux
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
            log::debug!("receiver remote cluster timeout");
            Ok(None)
        }
        #[cfg(windows)]
        Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
            log::debug!("receiver remote cluster timeout");
            Ok(None)
        }
        Err(e) => {
            log::warn!("received: {}", e);
            Ok(None)
        }
    };
}

/// 在[multicast_address]地址上监听，并发送服务端口给组播发送者, [signal]是关闭信息信号
pub async fn listener(
    multicast_address: SocketAddr,
    state: web::Data<AppState>,
    mut stopper: broadcast::Receiver<u64>,
) -> anyhow::Result<()> {
    assert!(
        multicast_address.ip().is_multicast(),
        "address is not multicast"
    );

    let bind_address = bind_address(&multicast_address, 0);
    log::info!("multicast bind address: {}", bind_address);

    let socket = TokioUdpSocket::bind(&bind_address).await?;

    let mut buf = [0u8; 4];
    match multicast_address.ip() {
        IpAddr::V4(ip) => socket.join_multicast_v4(ip.clone(), Ipv4Addr::UNSPECIFIED)?,
        IpAddr::V6(ip) => socket.join_multicast_v6(&ip, 0)?,
    };

    loop {
        tokio::select! {
            _ = stopper.recv() => {
                log::debug!("close listener");
                break;
            }
            output = socket.recv_from(&mut buf) => {
               if let Ok((len, remote)) = output {
                    let data = &buf[..len];
                    log::info!("multicast got data: {:?} from: {}", data, &remote);

                    let nodes = &state.nodes.read().unwrap();
                    if nodes.self_is_leader() && data.eq(&*SEARCH) {
                        let node = nodes.get_current().unwrap();
                        let mut send_data = [0u8;4];
                        send_data[0..2].copy_from_slice(&node.address.port().to_be_bytes()[0..2]);
                        send_data[2..].copy_from_slice(&node.id.to_be_bytes()[0..2]);
                        log::info!("send data: {:?} to {}", &send_data, &remote);
                        block_on(socket.send_to(&send_data, remote).then(|output| async move {
                            match output {
                                Ok(_) => {
                                    log::info!("Reply succeeded: {}", &remote);
                                }
                                Err(err) => {
                                    log::warn!("Multicast server {} sent response to: {}", remote, err);
                                }
                            }
                        }));
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn local_ipaddress() -> Option<IpAddr> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("8.8.8.8:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    return match socket.local_addr() {
        Ok(addr) => Some(addr.ip()),
        Err(_) => None,
    };
}

#[cfg(test)]
mod test {
    use std::net::SocketAddr;
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    use actix_web::web;
    use lazy_static::lazy_static;
    use tokio::sync::broadcast::{channel, Sender};

    use crate::cluster::multicast::{finder, listener};
    use crate::cluster::Node;
    use crate::config::logger;
    use crate::server::AppState;

    const PORT: u16 = 4567_u16;

    lazy_static! {
        static ref SIGNAL: Sender<u64> = {
            let (tx, _) = channel::<u64>(1);
            tx
        };
    }
    fn state() -> AppState {
        let state = AppState::default();
        state
            .nodes
            .write()
            .unwrap()
            .join(Node::new(0, format!("0.0.0.0:{}", PORT).parse().unwrap()))
            .set_current(0)
            .set_leader(Some(0));
        state
    }

    #[test]
    fn slave() {
        logger::init(true);
        thread::sleep(Duration::from_secs(1));
        let bind_address = SocketAddr::from_str("234.4.10.24:7657").unwrap();
        let timeout = Duration::from_secs(3);
        let (remote_address, id) = finder(&bind_address, Some(timeout))
            .expect("finder error")
            .expect("remote address is none");
        assert_eq!(PORT, remote_address.port());
        assert_eq!(0, id);
        assert_eq!(true, SIGNAL.send(1).is_ok());
    }

    #[tokio::test]
    async fn master() {
        logger::init(true);
        let address = SocketAddr::from_str("234.4.10.24:7657").unwrap();
        let state = web::Data::new(state());
        let _ = listener(address, state, SIGNAL.subscribe()).await;
    }
}
