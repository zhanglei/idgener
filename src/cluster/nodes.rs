use std::borrow::BorrowMut;
use std::net::SocketAddr;

use chrono::Utc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: u16,
    pub address: SocketAddr,
    pub last_alive_timestamp: i64,
}

impl Node {
    pub fn new(id: u16, address: SocketAddr) -> Self {
        Self {
            id,
            address,
            last_alive_timestamp: Utc::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nodes {
    current: Option<u16>,
    leader: Option<u16>,
    pub nodes: Vec<Node>,
}

/// 主机管理器
impl Nodes {
    /// 构建nodes管理器，
    pub fn default() -> Self {
        Nodes {
            leader: None,
            current: None,
            nodes: vec![],
        }
    }

    fn get_index(&self, id: u16) -> usize {
        let mut idx: usize = 0;
        for node in &self.nodes {
            if id <= node.id {
                return idx;
            }
            idx += 1
        }
        idx
    }

    ///返回是否已经初始化集群成功
    pub fn self_is_none(&self) -> bool {
        self.current.is_none()
    }

    pub fn get_current(&self) -> Option<&Node> {
        if let Some(current_id) = self.current {
            let idx = self.get_index(current_id);
            return self.nodes.get(idx);
        }
        None
    }

    pub fn get_leader(&self) -> Option<&Node> {
        if let Some(current_id) = self.leader {
            let idx = self.get_index(current_id);
            return self.nodes.get(idx);
        }
        None
    }

    /// 获取[id]的主机
    pub fn next(&self, id: u16) -> Option<&Node> {
        let idx = self.get_index(id);
        match self.nodes.get(idx + 1) {
            Some(node) => Some(node),
            None => self.nodes.get(0),
        }
    }

    pub fn get_node_by_address(&mut self, address: &SocketAddr) -> Option<&mut Node> {
        for node in &mut self.nodes {
            if node.address.eq(address) {
                return Some(node.borrow_mut());
            }
        }
        None
    }

    #[inline]
    pub fn join(&mut self, node: Node) -> &mut Self {
        let idx = self.get_index(node.id);
        self.nodes.insert(idx, node);
        self
    }

    pub fn new_node_id(&self) -> u16 {
        let mut node_id: u16 = 0;
        for node in &self.nodes {
            if node_id < node.id {
                return node_id;
            } else {
                node_id += 1
            }
        }
        node_id
    }

    pub fn is_leader(&self, node_id: u16) -> bool {
        match self.leader {
            Some(leader_id) => leader_id == node_id,
            None => false,
        }
    }

    #[inline]
    pub fn set_leader(&mut self, leader_id: Option<u16>) -> &mut Self {
        self.leader = leader_id;
        self
    }

    #[inline]
    pub fn set_current(&mut self, current_id: u16) -> &mut Self {
        self.current = Some(current_id);
        self
    }

    pub fn self_is_leader(&self) -> bool {
        match self.current {
            Some(current) => self.is_leader(current),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::cluster::{Node, Nodes};
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

    fn nodes() -> Nodes {
        let addr = "127.0.0.1:1024".parse::<SocketAddr>().unwrap();
        let mut nodes = Nodes {
            leader: Some(0_u16),
            current: Some(0_u16),
            nodes: vec![Node::new(0, addr)],
        };
        nodes.join(Node::new(
            nodes.new_node_id(),
            "127.0.0.1:8685".parse().unwrap(),
        ));
        nodes.join(Node::new(
            nodes.new_node_id(),
            "192.168.11.45:8081".parse().unwrap(),
        ));
        return nodes;
    }

    #[test]
    fn new_node() {
        let mut nodes = nodes();
        assert_eq!(3, nodes.nodes.len());

        let id = nodes.new_node_id();
        assert_eq!(3, id, "now is 3");

        nodes.join(Node::new(id, "127.0.0.1:8685".parse().unwrap()));
        nodes.join(Node::new(5, "127.0.0.2:8685".parse().unwrap()));

        let id = nodes.new_node_id();
        assert_eq!(4, id, "now is 4");

        nodes.join(Node::new(id, "127.0.0.3:8685".parse().unwrap()));

        let id = nodes.new_node_id();
        assert_eq!(6, id, "now is 6");
    }

    #[test]
    fn is_leader() {
        let mut nodes = nodes();
        assert_eq!(3, nodes.nodes.len());
        assert_eq!(true, nodes.self_is_leader());
        assert_eq!(false, nodes.is_leader(1));
        assert_eq!(false, nodes.is_leader(2));

        nodes.set_leader(Some(1));
        let node = nodes.get_leader().unwrap();
        assert_eq!(1, node.id);

        nodes.set_leader(Some(2));
        assert_eq!(false, nodes.self_is_leader());
        let node = nodes.get_leader().unwrap();
        assert_eq!(2, node.id);
    }

    #[test]
    fn set_leader() {
        let mut nodes = nodes();
        assert_eq!(true, nodes.self_is_leader());

        let current = nodes.get_current().unwrap();
        assert_eq!(0, current.id);

        nodes.set_leader(Some(2));
        assert_eq!(2, nodes.get_leader().unwrap().id);
    }

    #[test]
    fn test_none() {
        let mut nodes = Nodes::default();
        assert!(nodes.self_is_none());
        assert!(nodes.get_leader().is_none());
        assert!(nodes.get_current().is_none());

        nodes
            .join(Node::new(
                0,
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1024)),
            ))
            .set_leader(Some(0_u16))
            .join(Node::new(
                1,
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1025)),
            ))
            .set_current(1);

        assert_eq!(2, nodes.nodes.len());

        let leader = nodes.get_leader().expect("get leader error");
        assert_eq!(0, leader.id);
    }

    #[test]
    fn test_next() {
        let nodes = nodes();

        let n1 = nodes.next(0);
        assert!(n1.is_some());
        assert_eq!(1, n1.unwrap().id);

        let n1 = nodes.next(2);
        assert!(n1.is_some());
        assert_eq!(0, n1.unwrap().id);
    }
}
