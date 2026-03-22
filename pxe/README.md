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

Currently using `tgt` (Linux SCSI target framework) because VoE's
iscsi-server has a Data-Out PDU handling bug with the Linux open-iscsi
initiator. Once fixed, switch to VoE:

```bash
# tgt (working)
sudo tgtadm --lld iscsi --op new --mode target --tid 1 \
  -T iqn.2025-12.local.voe:storage.thinclient
sudo tgtadm --lld iscsi --op new --mode logicalunit --tid 1 \
  --lun 1 --backing-store /srv/iscsi/thinclient.img
sudo tgtadm --lld iscsi --op bind --mode target --tid 1 -I ALL

# VoE (once Data-Out bug is fixed)
sudo systemctl start iscsi-thinclient
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
