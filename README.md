# KeyInject

KeyInject sends trusted local keyboard and mouse-button input through Linux
`/dev/uinput`, including under Wayland. Start one foreground server; it owns
one persistent virtual input device and a private socket:

```sh
keyinject server
```

Then issue commands from the same login runtime directory:

```sh
keyinject input a          # tap
keyinject down leftctrl    # hold (idempotent)
keyinject up leftctrl      # release (idempotent)
keyinject input mouse-left
keyinject down BTN_MIDDLE
keyinject reset            # release every held input
```

`input` is a press/release tap and refuses an input already held by `down`.
Held state is global to the server, not a client. The server handles one
bounded request at a time; it has no daemon mode, movement, scrolling, text,
combinations, or macros. Keyboard names are case-insensitive; `KEY_` is
optional. Supported keys are letters, digits, `f1`–`f24`, editing/navigation
keys, arrows, and modifiers. Mouse aliases include `mouse-left`,
`mouse-middle`, `mouse-right`, `mouse1`–`mouse5`, `mouseforward`, and
`mouseback`, alongside `BTN_LEFT`, `BTN_MIDDLE`, `BTN_RIGHT`, `BTN_SIDE`,
`BTN_EXTRA`, `BTN_FORWARD`, and `BTN_BACK`.

The socket is always the absolute path `$XDG_RUNTIME_DIR/keyinject.sock`.
KeyInject requires `$XDG_RUNTIME_DIR` to be an owned, non-symlink directory
with exact mode `0700`; it requires its socket to be an owned Unix socket with
exact mode `0600` (no setuid, setgid, or sticky bits). A server never removes
or unlinks an existing socket pathname. If startup reports an existing
`keyinject.sock`, first verify no KeyInject server is running, then the same
trusted user who owns the runtime directory may inspect and remove it with
`rm -- "$XDG_RUNTIME_DIR/keyinject.sock"`. Do not use root or remove a socket
that may belong to a live server. Connections and requests time out, so a
client cannot indefinitely block the foreground serialized server. On a
normal server failure, it attempts `reset` before device cleanup; an abrupt
kill can leave a socket pathname for this manual recovery. If a client loses
its response, the outcome is unknown: use `reset` before continuing. The
no-uinput tests exercise this request/response and socket-path boundary; live
uinput injection requires separate authorization.

Client argument errors exit 2. Socket, server, and uinput failures exit 1.
The server stays in the foreground and reports operational errors to stderr.

## Security and permissions

Access to `/dev/uinput` is a powerful global-input capability: a process with
that access can synthesize shortcuts, input into password prompts, and
interact with lock screens. Run KeyInject only as a trusted local user. The
private runtime socket limits its command interface to that user, but does not
reduce the power of `/dev/uinput` itself.

KeyInject does not load kernel modules automatically, change permissions, or
install device rules. `/dev/uinput` must already be available to the trusted
user. Grant access through a narrow, administrator-managed device group or
equivalent restricted policy; do not make uinput or the socket world-writable
(including mode `0666`), install setuid bits, grant capabilities, add broad
permissions, or run a privileged daemon.
