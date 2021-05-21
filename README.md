# yap-frontend-cli
A frontend for YAP written in rust with the Cursive library, it uses XDG Base Directories and currently only supports theme configuration in `theme.toml`
## Building
On linux you can build the project with `cargo build --release`, the dependencies are `openssl` and of course `rust`. /
/
Building on windows wasn't tested but it should also work when using crossterm as the backend.
