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

    $(dirname $(realpath $0))/install-tbz2.sh --git nabijaczleweli/cargo-update --tag $tag --crate cargo-install-update
}

main
