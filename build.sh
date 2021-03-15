#!/bin/sh
cargo build --target aarch64-unknown-linux-gnu
docker build -t registry.undertheprinter.com/stomata:latest .
docker push registry.undertheprinter.com/stomata:latest
kubectl rollout restart deployment -n thyme stomata
