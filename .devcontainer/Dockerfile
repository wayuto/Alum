FROM quay.io/fedora/fedora:43

RUN dnf -y install \
      @development-tools git sudo fish \
      dnf clean all

RUN useradd -m -G wheel -s /usr/bin/fish vscode && \
    echo "%wheel ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/wheel
