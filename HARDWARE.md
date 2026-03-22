# NASty - Hardware Notes

## QNAP TS-412 "Floor12"

- **CPU**: Feroceon 88F6281 rev 1 (Marvell Kirkwood) @ 1.2 GHz
- **Architecture**: ARMv5TE — armel (soft-float, NOT armhf)
- **RAM**: 256 MB
- **Bootloader**: U-Boot (512 KB in flash)
- **Network**: 2x Gigabit Ethernet (eth0, eth1)
  - eth0: home network (10.0.1.x via DHCP)
  - eth1: PXE boot network (192.168.99.1 static)
- **USB**: 1x EHCI host controller
- **Crypto**: Marvell CESA hardware crypto engine (`mv_cesa`)
- **Disks**: 4-bay chassis, 2x 1TB SATA installed (bays 3-4 empty)
- **SSH**: `ssh floor12` (user matt)
- **OS**: Debian 10 (Buster) — last release supporting Kirkwood ARMv5TE

## Debian Installation

Installed via Martin Michlmayr's installer for Kirkwood-based QNAP devices.

1. Back up MTD flash (see below)
2. Download Michlmayr's installer and flash it
3. Reboot — installer runs via SSH (`ssh installer@floor12`, password `install`)
4. Complete Debian installer, selecting md9 as root

Note: Debian sees the Ethernet ports in reverse order from QTS.
eth0 under Debian is the port that was eth1 under QTS.

## MTD Flash Layout (16 MB NOR)

| Device | Size    | Contents       |
|--------|---------|----------------|
| mtd0   | 512 KB  | U-Boot         |
| mtd1   | 2 MB    | Kernel         |
| mtd2   | 9 MB    | RootFS1        |
| mtd3   | 3 MB    | RootFS2        |
| mtd4   | 256 KB  | U-Boot Config  |
| mtd5   | 1.25 MB | NAS Config     |

## Disk Partition Layout (per disk)

| Partition | Size   | Purpose              |
|-----------|--------|----------------------|
| p1        | 530 MB | md9 — Debian root    |
| p2        | 530 MB | md4 — swap           |
| p3        | ~930 GB| md0 — data/iSCSI     |
| p4        | 498 MB | md13 — unused        |

All partitions are mirrored as RAID1 (sda+sdb).

## MTD Backup (do this first!)

```bash
ssh floor12
for i in 0 1 2 3 4 5; do
  cat /dev/mtdblock${i} > /tmp/mtd${i}.bin
done
cd /tmp && tar czf qnap-mtd-backup.tar.gz mtd*.bin
```

Then from your workstation:
```bash
scp floor12:/tmp/qnap-mtd-backup.tar.gz .
```

## Recovery (nuclear option)

Power on with no valid OS and U-Boot requests firmware via TFTP.
Stock firmware: https://eu1.qnap.com/Storage/TS-412/TS-412_20240619-4.3.3.2784.zip

## QEMU Test Environment

CRITICAL: target is **armel** (ARMv5TE soft-float), NOT armhf.

### TAP network setup (run once)

```bash
sudo ip tuntap add dev tap10 mode tap user matt
sudo ip addr add 10.0.101.1/24 dev tap10
sudo ip link set tap10 up
```

### Create disk and fetch installer

```bash
sudo apt install qemu-system-arm
qemu-img create -f qcow2 /opt/isos/nasty-armel.qcow2 16G
wget http://ftp.debian.org/debian/dists/buster/main/installer-armel/current/images/netboot/initrd.gz
wget http://ftp.debian.org/debian/dists/buster/main/installer-armel/current/images/netboot/vmlinuz
```

### Install (boot from installer)

```bash
qemu-system-arm -M virt -m 256M \
  -kernel vmlinuz -initrd initrd.gz \
  -drive file=/opt/isos/nasty-armel.qcow2,if=virtio \
  -nographic \
  -append "console=ttyAMA0" \
  -netdev tap,id=net0,ifname=tap10,script=no,downscript=no \
  -device virtio-net-device,netdev=net0
```

### Run (after install)

Save as `/opt/isos/nasty.sh`:

```bash
#!/bin/sh

# NASty - Debian Buster armel test VM
#
# Network: tap10 (10.0.101.0/24)
#   VM IP: 10.0.101.2
#   Host:  10.0.101.1
#
# In VM: configure /etc/network/interfaces with static 10.0.101.2/24
# iSCSI test: iscsiadm from host pointing at 10.0.101.2:3260

qemu-system-arm -M virt -m 256M \
  -name nasty \
  -drive file=/opt/isos/nasty-armel.qcow2,if=virtio \
  -nographic \
  -append "console=ttyAMA0 root=/dev/vda1" \
  -netdev tap,id=net0,ifname=tap10,script=no,downscript=no \
  -device virtio-net-device,netdev=net0
```

Note: `-M virt` doesn't emulate Kirkwood specifically, but is fine for
testing armel binaries. Use 256M to match real hardware constraints.
The tap network lets the host connect to the VM's iSCSI port directly.
