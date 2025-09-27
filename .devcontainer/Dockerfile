FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

# Build args (can be overridden by devcontainer.json or docker build --build-arg)
ARG DEBIAN_FRONTEND=noninteractive
ARG LLVM_VERSION=21
ARG RUST_VERSION=1.90
ARG INSTALL_RUST=true

# Export sensible runtime envs
ENV DEBIAN_FRONTEND=${DEBIAN_FRONTEND}
ENV LLVM_VERSION=${LLVM_VERSION}
ENV RUST_VERSION=${RUST_VERSION}
ENV PATH=/root/.cargo/bin:/usr/lib/llvm-${LLVM_VERSION}/bin:${PATH}

RUN set -eux \
    # Basic deps used by the installers and general dev tools
    && apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gnupg \
        lsb-release \
        build-essential \
        cmake \
        pkg-config \
        git \
        python3 \
        python3-pip \
        sudo \
        wget \
        software-properties-common \
    && rm -rf /var/lib/apt/lists/*

# Install LLVM via the official llvm.sh helper (use 'all' to install the full toolchain)
RUN set -eux; \
    curl -fsSL https://apt.llvm.org/llvm.sh -o /tmp/llvm.sh; \
    chmod +x /tmp/llvm.sh; \
    # the helper will add the correct apt repo for this distro and install
    /tmp/llvm.sh "${LLVM_VERSION}" all; \
    rm -f /tmp/llvm.sh; \
    apt-get update; \
    rm -rf /var/lib/apt/lists/*;

# Configure environment to prefer the requested LLVM binaries, libraries and headers.
ENV LD_LIBRARY_PATH=/usr/lib/llvm-${LLVM_VERSION}/lib
ENV LIBRARY_PATH=/usr/lib/llvm-${LLVM_VERSION}/lib
ENV C_INCLUDE_PATH=/usr/lib/llvm-${LLVM_VERSION}/include
ENV CPLUS_INCLUDE_PATH=/usr/lib/llvm-${LLVM_VERSION}/include
ENV CPATH=/usr/lib/llvm-${LLVM_VERSION}/include

# Optionally install rustup and the requested Rust toolchain (controlled by INSTALL_RUST build arg)
RUN if [ "${INSTALL_RUST}" = "true" ]; then \
        set -eux; \
        curl -sSf https://sh.rustup.rs -o /tmp/rustup-init.sh; \
        chmod +x /tmp/rustup-init.sh; \
        /tmp/rustup-init.sh -y --default-toolchain "${RUST_VERSION}"; \
        . /root/.cargo/env; \
        rustup default "${RUST_VERSION}"; \
        rustup component add rust-src rustfmt clippy || true; \
        rm -f /tmp/rustup-init.sh; \
        rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
    else \
        echo "Skipping Rust installation (INSTALL_RUST=${INSTALL_RUST})"; \
    fi

CMD ["/bin/bash"]
