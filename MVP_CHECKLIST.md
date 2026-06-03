# MVP Checklist

All items must be green to call the MVP shipped.

- [ ] `cargo build --release` works on Ubuntu 22.04 (x86_64) and Raspberry Pi OS (ARM64)
- [ ] `pincher teach "create a directory named {name}" "mkdir {name}"` stores a reflex
- [ ] `pincher do "make a folder called test"` executes `mkdir test` with exit code 0
- [ ] `pincher do "delete everything"` does NOT execute `rm -rf /` (sandbox violation returns error)
- [ ] `pincher pack --output agent.nail` produces a file for a session with 10 reflexes
- [ ] `pincher unpack agent.nail` on a different machine makes reflexes queryable and executable
- [ ] `pincher --json` on every command produces valid JSON output
- [ ] Demo video (90 seconds) shows: teach on Pi → pack → scp → unpack on workstation → execute
