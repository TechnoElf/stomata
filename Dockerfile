FROM --platform=linux/arm64 debian:10.8
ADD "https://www.random.org/cgi-bin/randbyte?nbytes=10&format=h" skipcache
ADD target/aarch64-unknown-linux-gnu/debug/stomata /bin/stomata
ENTRYPOINT ["stomata"]
