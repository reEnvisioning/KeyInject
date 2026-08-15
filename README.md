# KeyInject

Trusted local keyboard and mouse-button injection through Linux `/dev/uinput`.

## Install and use

```sh
nix run github:reEnvisioning/KeyInject -- server   # foreground server
keyinject input a                                  # tap
keyinject down leftctrl                            # hold
keyinject up leftctrl                              # release
keyinject input mouse-left
keyinject reset                                    # release all held inputs
```

Names are case-insensitive; `KEY_` is optional. Supported inputs include
letters, digits, F1–F24, editing/navigation keys, arrows, modifiers, and common
mouse buttons. There is no text, macro, pointer-motion, scrolling, daemon, or
compositor IPC mode.

## Compatibility

Linux only. uinput feeds the kernel input subsystem, so it is independent of
Niri, Hyprland, X11, or a Wayland compositor. The user must already have
restricted read/write access to `/dev/uinput`; errors state when that capability
is missing. The private client socket is `$XDG_RUNTIME_DIR/keyinject.sock`.

## Security

**uinput access can inject into shortcuts, password prompts, and lock screens.**
Run only as a trusted local user. KeyInject adds no setuid binary, capability,
udev rule, privileged service, permission change, or kernel-module action.
Never make `/dev/uinput` or the socket world-writable.

The runtime directory must be owned, non-symlink, absolute, and mode `0700`;
the socket is mode `0600`. If a crash leaves a socket, first verify no server
is running, then its owning user may remove it. After an uncertain response,
run `keyinject reset`.
