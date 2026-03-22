# NASty - Project Plan

## Goal

Replace QTS on the QNAP TS-412 with minimal Debian, serving as:

1. **iSCSI target** — VoE's `iscsi-server` backed by file storage on the RAID array
2. **PXE boot server** — diskless thin clients boot Debian over iSCSI from Floor12

No web UI, no services, no bloat.

## Architecture

Three repos work together:

- **iscsi-crate** — Pure Rust iSCSI target library. Implements the iSCSI
  protocol and exposes a `ScsiBlockDevice` trait for pluggable backends.
- **VoE** — Application layer. Provides the `iscsi-server` binary with
  file-backed and CAS-backed `ScsiBlockDevice` implementations.
- **NASty** (this repo) — Deployment: getting minimal Debian onto the QNAP
  hardware, PXE boot server configuration, thin client image.

## What Runs on the QNAP

- **iscsi-server** — file-backed iSCSI target on port 3260 (192.168.99.1)
- **dnsmasq** — DHCP + TFTP + DNS on eth1 for PXE clients
- **python3 http.server** — serves kernel+initrd to iPXE (port 8080)
- **nftables** — NAT/masquerade between eth1 (PXE) and eth0 (home)
- Minimal Debian (sshd, mdadm, networking)

## OS

Debian 10 (Buster) — the last Debian release supporting Marvell Kirkwood
ARMv5TE. Installed via Martin Michlmayr's Kirkwood QNAP installer.

## Network Layout

```
eth0 (home):  10.0.1.x/24 via DHCP (router at 10.0.1.1)
eth1 (PXE):   192.168.99.1/24 static
              DHCP pool: 192.168.99.100-199
              IP forwarding + NAT between eth0 and eth1
```

## PXE Boot Flow

1. Thin client PXE ROM broadcasts DHCP
2. dnsmasq responds with IP + `ipxe.efi` (UEFI) or `undionly.kpxe` (BIOS)
3. iPXE fetches `autoexec.ipxe` via TFTP
4. Boot script fetches kernel + initrd via HTTP (port 8080)
5. Kernel boots with iSCSI parameters, initramfs connects to iSCSI target
6. Root filesystem mounted from iSCSI LUN, Debian boots

## Thin Client Image

- 100GB sparse file at `/srv/iscsi/thinclient.img` (backup at `.bak`)
- GPT: 512MB EFI System Partition + 99.5GB ext4 root
- Debian 13 (Trixie) amd64
- Includes: openssh-server, wpasupplicant, firmware-iwlwifi, open-iscsi
- Built with debootstrap + grub-efi-amd64 on workstation

## Cross-Compilation (for VoE on Floor12)

```bash
rustup target add armv5te-unknown-linux-musleabi
sudo apt install gcc-arm-linux-gnueabi

CARGO_TARGET_ARMV5TE_UNKNOWN_LINUX_MUSLEABI_LINKER=arm-linux-gnueabi-gcc \
  cargo build --release --target armv5te-unknown-linux-musleabi --bin iscsi-server
```

## Current Status

- [x] MTD backup
- [x] Debian Buster installed on Floor12
- [x] PXE boot server (dnsmasq + iPXE + iSCSI) working
- [x] Thin client boots Debian Trixie over iSCSI (using tgt)
- [x] IP forwarding + NAT between PXE and home networks
- [ ] VoE iscsi-server Data-Out bug with Linux initiator (works with tgt)
- [ ] Make tgt config persistent across reboots
- [ ] Thin client WiFi configuration
- [ ] HTTP server as systemd service (currently manual python3)

## Storage Layout

- **md9** (sda1+sdb1, 530 MB) — Debian root filesystem
- **md4** (sda2+sdb2, 530 MB) — swap
- **md0** (sda3+sdb3, ~930 GB) — iSCSI backing store
- md13 can be repurposed or left unused
