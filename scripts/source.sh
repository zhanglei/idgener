#!/usr/bin/env bash

info() {
  echo "::info $*" >&2
}

error() {
  echo "::error file=build.sh:: $*" >&2
}

crash() {
  error "Command exited with non-zero exit code"
  exit 1
}

export api="https://api.github.com/repos/ihaiker/idgener"
export repo="https://github.com/ihaiker/idgener"

export this_tag=$(git describe --abbrev=0 --tags `git rev-list --tags --max-count=1`)
export this_tag=${GIT_TAG_NAME:-$this_tag}
export previous_tag=$(git describe --abbrev=0 --tags `git rev-list --tags --skip=1 --max-count=1`)

export -f info
export -f error
export -f crash
