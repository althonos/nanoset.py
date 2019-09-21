#!/bin/sh -e

. $(dirname $(dirname $0))/functions.sh

# --- Install Rust -----------------------------------------------------------

log Installing Rust nightly on local machine
curl -sSf https://build.travis-ci.org/files/rustup-init.sh | sh -s -- --default-toolchain=nightly -y


# --- Check versions ---------------------------------------------------------

log Checking versions in package metadata
python ci/vercheck.py
