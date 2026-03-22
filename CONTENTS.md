# NASty - Project Documentation

## Contents

- PLANNING.md  - Architecture, phases, and deployment plan
- HARDWARE.md  - Hardware specs, partition layout, MTD backup, QEMU setup
- CONTENTS.md  - This file
- pxe/         - PXE boot server configuration

## Overview

NASty is a minimal Debian deployment for the QNAP TS-412 "Floor12",
replacing QTS with just enough OS to run a Rust iSCSI target from the
VoE project. Additionally, Floor12 serves as a PXE boot server for
diskless thin clients over iSCSI.

## Related Repos

- **../iscsi-crate** — Pure Rust iSCSI target library (`ScsiBlockDevice` trait)
- **../VoE** — iSCSI server binary (depends on iscsi-crate)
