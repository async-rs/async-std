set -euxo pipefail

# Based on the Rust-Embedded WG's book CI
# https://github.com/rust-embedded/book/blob/master/ci/install.sh

main() {
    curl -LSfs https://japaric.github.io/trust/install.sh | \
        sh -s -- --git rust-lang-nursery/mdbookk
}

main
