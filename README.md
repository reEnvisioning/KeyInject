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

## Use

```sh
keyinject server
keyinject input a
keyinject down leftctrl
keyinject up leftctrl
keyinject reset
```

## Safety

Global input injection is for trusted local users only. Linux and restricted read/write `/dev/uinput` access are required; never make it world-writable.
