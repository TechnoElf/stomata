#!/bin/sh
docker run --rm --user "$(id -u)":"$(id -g)" -v "$PWD":/tmp/build -v ~/.cargo/:"/home/$(whoami)/.cargo/" -w /tmp/build registry.undertheprinter.com/rust-arm-cross:latest cargo build --target aarch64-unknown-linux-gnu
docker build -t registry.gitlab.com/heimdallr1/heimdallr-server/stomata:latest .
docker push registry.gitlab.com/heimdallr1/heimdallr-server/stomata:latest
kubectl rollout restart deployment -n thyme stomata
