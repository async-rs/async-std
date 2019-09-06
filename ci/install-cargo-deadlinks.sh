set -euxo pipefail

# Based on the Rust-Embedded WG's book CI
# https://github.com/rust-embedded/book/blob/master/ci/install.sh

main() {
    # Note - this will only accept releases tagged with v0.4.x
    local tag=$(git ls-remote --tags --refs --exit-code \
                    https://github.com/deadlinks/cargo-deadlinks \
                        | cut -d/ -f3 \
                        | grep -E '^0\.4\.[0-9]+$' \
                        | sort --version-sort \
                        | tail -n1)

    curl -LSfs https://japaric.github.io/trust/install.sh | \
        sh -s -- --git deadlinks/cargo-deadlinks --tag $tag
}

main
