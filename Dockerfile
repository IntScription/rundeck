FROM rust:1.94-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY nvim ./nvim
COPY README.md LICENSE ./

RUN cargo build --release


FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="RunDeck"
LABEL org.opencontainers.image.description="Terminal dashboard for personal dev projects"
LABEL org.opencontainers.image.source="https://github.com/IntScription/rundeck"
LABEL org.opencontainers.image.licenses="MIT"

ARG LAZYGIT_VERSION=0.44.1

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    fzf \
    git \
    neovim \
    tar \
    tmux \
    zsh \
    && rm -rf /var/lib/apt/lists/*

RUN arch="$(uname -m)" \
    && case "$arch" in \
      x86_64) lazygit_arch="x86_64" ;; \
      aarch64|arm64) lazygit_arch="arm64" ;; \
      *) echo "Unsupported architecture: $arch" && exit 1 ;; \
    esac \
    && curl -fsSL "https://github.com/jesseduffield/lazygit/releases/download/v${LAZYGIT_VERSION}/lazygit_${LAZYGIT_VERSION}_Linux_${lazygit_arch}.tar.gz" -o /tmp/lazygit.tar.gz \
    && tar -xzf /tmp/lazygit.tar.gz -C /tmp lazygit \
    && install -m 755 /tmp/lazygit /usr/local/bin/lazygit \
    && rm -f /tmp/lazygit /tmp/lazygit.tar.gz

COPY --from=builder /app/target/release/rundeck /usr/local/bin/rundeck

ENV SHELL=/bin/zsh
ENV TERM=xterm-256color

WORKDIR /workspace

ENTRYPOINT ["rundeck"]
CMD ["doctor"]
