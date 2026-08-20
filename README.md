# KeyInject

Trusted local keyboard and mouse-button injection through Linux `/dev/uinput`.

## Install

```sh
cargo install --git https://github.com/reEnvisioning/KeyInject.git
nix run github:reEnvisioning/KeyInject -- server
```

```nix
inputs.keyinject.url = "github:reEnvisioning/KeyInject";
packages.${pkgs.system}.default = inputs.keyinject.packages.${pkgs.system}.default;
```

## Permissions

NixOS example:

```nix
hardware.uinput.enable = true;
users.users.visionary.extraGroups = [ "uinput" ]; # trusted account only
```

On Void or Arch, enable `/dev/uinput` and grant read/write access only to the trusted local user or a small trusted group. Start a fresh login afterward; do not run `keyinject` with sudo.

## Use

In terminal one:

```sh
keyinject server
```

In terminal two:

```sh
keyinject input a
keyinject down leftctrl
keyinject up leftctrl
keyinject reset
```

## Safety

Global input injection is for trusted local users only. Linux and restricted read/write `/dev/uinput` access are required; never make it world-writable.
The CLI is distro-neutral across NixOS, Void, and Arch, runs in the foreground, and does not require systemd; use a shell, runit, or any supervisor you trust. macOS builds only keep help/parsing available; injection is Linux-only.
If the server is interrupted, the next `keyinject server` removes a refused same-user mode-0600 socket at its runtime path.

Run `keyinject help` for the full command reference. `keyinject available` only probes `/dev/uinput`; it creates no socket or virtual device.
