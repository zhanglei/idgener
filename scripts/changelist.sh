#!/bin/bash
set -e

export BASE_PATH="$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )"
cd $BASE_PATH
source $BASE_PATH/source.sh

info "Tag from ($previous_tag...$tag]"

cat <<EOF | tee $BASE_PATH/header.md

## What  Different
[$previous_tag...$tag](${repo}/compare/$previous_tag...$tag)

## Full Changelog
$(git log --pretty="format:- [\[%t\]]($repo/commit/%T) %s" --no-merges "$previous_tag...$tag")

EOF
