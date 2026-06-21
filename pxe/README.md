# PXE Boot Server Configuration

Floor12 serves as a PXE boot server for diskless thin clients.
Thin client boots Debian Trixie over iSCSI from Floor12's second
Ethernet port.

## Network

- **eth1** (192.168.99.1/24) — dedicated PXE/iSCSI link to thin client
- **eth0** (10.0.1.x) — home network, unchanged
- Floor12 routes and NATs between the two

## Files

| File | Deploys to | Purpose |
|------|------------|---------|
| `interfaces-eth1` | `/etc/network/interfaces.d/eth1` | Static IP for PXE port |
| `dnsmasq.conf` | `/etc/dnsmasq.d/pxe.conf` | DHCP + TFTP + DNS |
| `boot.ipxe` | `/srv/tftp/boot.ipxe` | iPXE script (kernel+initrd via HTTP) |
| `iscsi-thinclient.service` | `/etc/systemd/system/` | VoE iSCSI target service |
| `sysctl-forward.conf` | `/etc/sysctl.d/10-forward.conf` | Enable IP forwarding |
| `nftables-nat.conf` | `/etc/nftables.d/pxe-nat.conf` | Masquerade PXE→home |

## TFTP Root (/srv/tftp/)

| File | Source |
|------|--------|
| `ipxe.efi` | Built from ipxe source (`make bin-x86_64-efi/ipxe.efi`) |
| `undionly.kpxe` | http://boot.ipxe.org/undionly.kpxe (BIOS clients) |
| `boot.ipxe` | This repo |
| `autoexec.ipxe` | Symlink → boot.ipxe |
| `vmlinuz` | Extracted from thin client image |
| `initrd.img` | Extracted from thin client image |

## Boot Flow

```
PXE ROM → DHCP (dnsmasq) → TFTP ipxe.efi
       → iPXE fetches autoexec.ipxe via TFTP
       → iPXE fetches vmlinuz + initrd.img via HTTP :8080
       → Kernel boots, initramfs connects to iSCSI (192.168.99.1:3260)
       → Root filesystem mounted, Debian Trixie boots
```

## Thin Client Image

Located at `/srv/iscsi/thinclient.img` on Floor12.

- 100GB sparse (GPT + EFI + ext4)
- Debian 13 (Trixie) amd64
- Built with debootstrap on workstation, transferred via zstd

### Building the image

```bash
truncate -s 100G /tmp/thinclient.img
sfdisk /tmp/thinclient.img <<EOF
label: gpt
2048,512M,U
,+,L
EOF
losetup -P --find --show /tmp/thinclient.img  # e.g. /dev/loop0
mkfs.vfat -F 32 -n EFI /dev/loop0p1
mkfs.ext4 -L thinclient /dev/loop0p2
mount /dev/loop0p2 /mnt/thinclient
mkdir -p /mnt/thinclient/boot/efi
mount /dev/loop0p1 /mnt/thinclient/boot/efi
debootstrap --arch=amd64 trixie /mnt/thinclient http://deb.debian.org/debian
# chroot, install: linux-image-amd64 grub-efi-amd64 openssh-server
#                  wpasupplicant firmware-iwlwifi open-iscsi locales
# grub-install --target=x86_64-efi --efi-directory=/boot/efi --removable --no-nvram
# update-grub
```

### Transferring to Floor12

```bash
zstd -1 /tmp/thinclient.img -o /tmp/thinclient.img.zst
scp /tmp/thinclient.img.zst floor12:/tmp/
ssh floor12 'sudo zstd -d /tmp/thinclient.img.zst -o /srv/iscsi/thinclient.img'
```

## iSCSI Target

Served by VoE's `iscsi-server`, run as `iscsi-thinclient.service`. It binds
`192.168.99.1:3260` and exports `/srv/iscsi/thinclient.img` as target
`iqn.2025-12.local.voe:storage.thinclient`.

```bash
sudo systemctl status iscsi-thinclient    # check
sudo systemctl restart iscsi-thinclient   # bounce after a binary swap
sudo journalctl -u iscsi-thinclient -f    # logs (RUST_LOG=debug in the unit)
```

The old `tgt` stopgap has been retired (`systemctl disable --now tgt`,
2026-06-21). VoE was blocked by a Data-Out PDU bug with the Linux open-iscsi
initiator; that is fixed as of `iscsi-crate` 1.0.0 (verified with a 32 MB
multi-PDU write/read-back through open-iscsi).

### Rebuilding and redeploying iscsi-server

Cross-compile a static ARMv5 musl binary on the workstation, ship it, swap it:

```bash
# in ../VoE
CARGO_TARGET_ARMV5TE_UNKNOWN_LINUX_MUSLEABI_LINKER=arm-linux-gnueabi-gcc \
  cargo build --release --target armv5te-unknown-linux-musleabi --bin iscsi-server

scp target/armv5te-unknown-linux-musleabi/release/iscsi-server floor12:/tmp/
ssh floor12 'sudo install -m755 /tmp/iscsi-server /usr/local/bin/iscsi-server \
  && sudo systemctl restart iscsi-thinclient'
```

## Deployment

```bash
# On Floor12:
sudo cp interfaces-eth1 /etc/network/interfaces.d/eth1
sudo cp dnsmasq.conf /etc/dnsmasq.d/pxe.conf
sudo cp sysctl-forward.conf /etc/sysctl.d/10-forward.conf
sudo cp nftables-nat.conf /etc/nftables.d/pxe-nat.conf
sudo mkdir -p /srv/tftp
sudo sysctl -p /etc/sysctl.d/10-forward.conf
sudo nft -f /etc/nftables.d/pxe-nat.conf
sudo ifup eth1
sudo systemctl restart dnsmasq
```
