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
ENV PATH=/root/.cargo/bin:/home/vscode/.cargo/bin:/usr/lib/llvm-${LLVM_VERSION}/bin:${PATH}

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
        gdb \
        valgrind \
    && rm -rf /var/lib/apt/lists/*

# Install GitHub CLI
RUN set -eux; \
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg; \
    chmod go+r /usr/share/keyrings/githubcli-archive-keyring.gpg; \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" | tee /etc/apt/sources.list.d/github-cli.list > /dev/null; \
    apt-get update; \
    apt-get install -y gh; \
    rm -rf /var/lib/apt/lists/*;

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
        # Ensure sudo exists so we can run the rustup installer as the vscode user during build
        apt-get update; apt-get install -y --no-install-recommends sudo; \
        # Install rustup for the 'vscode' user so the toolchain is available to the non-root workspace user
        sudo -u vscode bash -lc "curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain ${RUST_VERSION}"; \
        # Use the user's cargo environment to install components
        sudo -u vscode bash -lc "export PATH=\"$HOME/.cargo/bin:$PATH\"; rustup default ${RUST_VERSION}; rustup component add rust-src rustfmt clippy || true"; \
        rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*; \
    else \
        echo "Skipping Rust installation (INSTALL_RUST=${INSTALL_RUST})"; \
    fi

CMD ["/bin/bash"]
