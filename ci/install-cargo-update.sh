set -euxo pipefail

# Based on the Rust-Embedded WG's book CI
# https://github.com/rust-embedded/book/blob/master/ci/install.sh

main() {
    # Note - this will accept any tagged release
    local tag=$(git ls-remote --tags --refs --exit-code \
                    https://github.com/nabijaczleweli/cargo-update \
                        | cut -d/ -f3 \
                        | grep -E '^v[0-9\.]+$' \
                        | sort --version-sort \
                        | tail -n1)

    curl -LSfs https://japaric.github.io/trust/install.sh | \
        sh -s -- --git nabijaczleweli/cargo-update --tag $tag
}

main
