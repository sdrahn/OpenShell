# openshell-driver-systemd

A compute driver that runs each sandbox as systemd units on the host of the
OpenShell gateway, without a container runtime or a VM. "The gateway" below
is always the OpenShell gateway. It uses the stock
`openshell-supervisor` and `openshell-sandbox` unchanged: policy, Landlock,
seccomp and network mediation are theirs, as with every other driver.

This driver is for Linux hosts where the workload should use the host's own
installed tools, and where systemd units and the host's security policy are
how services are run and confined.

## What a sandbox is

Three instances of unit templates, named by the sandbox's ID:

| Unit | Runs | As | Network |
|---|---|---|---|
| `openshell-sandbox-netns@ID.service` | holds the sandbox's network namespace | root | a namespace with loopback only |
| `openshell-sandbox-supervisor@ID.service` | `openshell-supervisor` | `openshell-sandbox` | the host's |
| `openshell-sandbox-workload@ID.service` | `openshell-sandbox`, which starts the agent | `openshell-sandbox` | the namespace of the unit above |

The outer fence is the network namespace: the workload has no interface but
loopback, so all egress goes through the supervisor's control socket. Both
units have no capabilities and `NoNewPrivileges=yes`. The workload also has
a process ID namespace of its own (`PrivatePIDs=`, systemd 257 or later):
every sandbox and supervisor runs as one user, and without it an agent could
signal the processes of another sandbox.

The workload's root file system is the host's, read-only. A sandbox of this
driver has no image; a request that names one is refused.

## Two processes

The gateway's driver socket accepts only the gateway's own user, so the
driver runs as that user and cannot start system units. The crate therefore
builds two programs:

- `openshell-driver-systemd` is the `ComputeDriver` service. It builds a
  launch's bootstrap file, runtime descriptor and TLS material, as the
  container drivers do, and has no privilege.
- `openshell-systemd-helper` runs as root. It writes a launch's files to a
  directory of its own and starts, stops and reads the state of the three
  units. It serves one user, checked from its socket's peer credentials.

The helper is the privilege boundary. A request carries file contents and a
sandbox ID, never a path, a unit name or a command. The ID must be a
canonical UUID and is the only part of a request that reaches a path or a
unit name. The units themselves are the templates in `systemd/`, installed
with the package, which an administrator can read and override with
drop-ins.

The protocol is one JSON request and one JSON response per connection
(`src/protocol.rs`).

## Files of a sandbox

`/var/lib/openshell/systemd/ID/`, written by the helper:

| Path | Owner, mode | Mounted in |
|---|---|---|
| `channel/sandbox/` (bootstrap file, TLS identity) | workload user, 0711 | both, read-only for the supervisor |
| `control/` (the control socket) | root, 1777 | both |
| `supervisor/` (runtime descriptor, auth bundle) | workload user, 0700 | the supervisor only |
| `etc/` (gateway token, gateway CA) | workload user, 0700 | the supervisor only |
| `supervisor.env` | root, 0600 | read by systemd |
| `workspace/` | workload user, 0700 | the workload, at `/sandbox` |

The supervisor and the workload run as the same user, as in the Podman
driver. What keeps the supervisor's credentials from the agent is the
workload's mount namespace, which does not contain them, and Landlock: the
workload's unit mounts an empty file system over `/var/lib/openshell` and
over `/run/openshell-sandbox`, where each supervisor's sockets and proxy
keys are, and binds in the channel and the workspace only.

## Setup

1. Create the workload user: `useradd --system --user-group openshell-sandbox`.
2. Install `openshell-supervisor` and `openshell-sandbox` in
   `/usr/libexec/openshell/`, with `openshell-systemd-helper`.
3. Install the units of `systemd/` in `/usr/lib/systemd/system/`.
4. Name the gateway's user for the helper in a drop-in and start it:

   ```shell
   systemctl edit openshell-systemd-helper.service   # Environment=OPENSHELL_SYSTEMD_HELPER_ALLOW_USER=<user>
   systemctl enable --now openshell-systemd-helper.service
   ```

5. Run the driver as the gateway's user and point the gateway at it:

   ```shell
   openshell-driver-systemd --bind-socket "$XDG_RUNTIME_DIR/openshell/systemd-driver.sock" \
       --gateway-tls-ca <the gateway's CA certificate>
   ```

   ```toml
   [openshell.gateway]
   compute_driver = "systemd"

   [openshell.drivers.systemd]
   socket_path = "/run/user/1000/openshell/systemd-driver.sock"
   ```

The helper creates the mount points the units bind to (`/.openshell`,
`/sandbox`, `/etc/openshell`), which are the paths the stock programs use in
a container.

## Not supported

- Images: there is no root file system but the host's.
- CPU, memory and GPU requests. `TasksMax=` in the workload's template is
  the only limit; set others with drop-ins.
- Suspend and resume, and sandbox authentication.
- A workload user per sandbox: every sandbox runs as `openshell-sandbox`. A
  policy that asks for another user or group is refused.
- An in-process driver in the gateway: this driver is an external one.
