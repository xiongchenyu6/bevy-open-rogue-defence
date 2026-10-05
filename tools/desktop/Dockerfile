# Steam Runtime 3 SDK, resolved and pinned on 2026-10-06.
FROM registry.gitlab.steamos.cloud/steamrt/sniper/sdk@sha256:1c33c507bc75d012e77df5727f93b0d5b8c3f7c8d4142ba5f7a16882cc92e014
ENV RUSTUP_HOME=/opt/rogue-rustup CARGO_HOME=/opt/rogue-cargo
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rogue-rustup.sh \
    && sh /tmp/rogue-rustup.sh -y --profile minimal --default-toolchain 1.98.0 --no-modify-path \
    && rm /tmp/rogue-rustup.sh
ENV PATH=/opt/rogue-cargo/bin:$PATH
