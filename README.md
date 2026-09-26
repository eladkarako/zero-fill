<h3><img width="48" src="resources/app.png" /> <code>zero-fill</code></h3>

## safely allocate and release filesystem space on Linux and Android using `fallocate`


<hr/>

I wrote this to mostly improve storage of vm and lite-vm vmdk virtual-hd for backups,  
on Windows, after running it on your WSL (or vm),  
you can optimize it even more but first for wsl: `wsl --shutdown`

```cmd
diskpart

list vdisk
(There are no virtual disks to show.)

select vdisk file="E:\wsl\ext4.vhdx"
(DiskPart successfully selected the virtual disk file.)

compact vdisk
(10 percent completed .. 100 percent completed)

exit

wsl --export Ubuntu "E:\wsl\202609262020_ubuntu.tar"

```

<hr/>

before importing, try `wsl --update --pre-release`

you can try `sparseVhd=true` under `[experimental]` in `C:\Users\[USERNAME]\.wslconfig`  

(..better reboot)

for example:  

```ini
[wsl2]
processors=4
swap=2147483648
firewall=false
guiApplications=false
nestedVirtualization=false
[experimental]
sparseVhd=true
```

<hr/>

### started as...

trying to improve 

```bash
sudo dd if=/dev/zero of=zero.fill bs=1M
sudo sync
sudo rm zero.fill
```

to  

```bash
sudo sh -c '
  trap "rm -f -- zero.fill" EXIT
  dd if=/dev/zero of=zero.fill bs=1M status=progress
  sync
'
```

This:
- Shows progress.
- Automatically removes `zero.fill` even if interrupted or dd exits with `No space left on device`.
- Runs the `creation`, `sync`, and `deletion` with the required permissions.
- Avoids leaving a `root`-owned file behind.

to  

simpler

```bash
sudo dd if=/dev/zero of=zero.fill bs=1M status=progress || true
sudo sync
sudo rm -f -- zero.fill
```

since it is useful for overwriting currently free space with zeros, but it is not reliable for securely erasing deleted data on SSDs or flash storage because wear-leveling can leave old blocks intact.

<hr/>

to Rust...  
with a shameful loop..
somewhere was `28`.. which is Linux's `ENOSPC` ("No space left on device").

<hr/>

to Rust...  
with `fallocate(2)` .  

which is a better fit than repeatedly writing a zero buffer,  
It asks the filesystem to allocate and initialize blocks directly,  
avoiding user-space zero copying.

it is supported by common Linux filesystems such as ext4 and XFS,  
but behavior can vary on filesystems that do not support allocation.  

`fallocate` may allocate blocks without physically writing every zero to the storage medium,  
depending on the filesystem and device.  

For SSDs, neither this nor the original method reliably erases previously deleted data.

note: I use `tokio` (multi-threading) version since it uses `spawn_blocking` because `fallocate` is a blocking system call:

<hr/>

to Rust..  
with a safer modification
- Creating a unique temporary file with exclusive creation, so it cannot overwrite an existing `zero.fill`.
- Giving it restrictive permissions.
- Keeping a configurable amount of free space instead of filling the filesystem completely.
- Automatically deleting the file when the operation finishes or returns an error.
- Using `fallocate` through `spawn_blocking`, since it is blocking system `I/O`.


note: `tokio` is used with an async coordinator, `spawn_blocking` for the blocking Linux allocation calls,  
and an async channel to send progress updates back to the main task.

This version:
- Performs allocation on a worker thread.
- Keeps the Tokio runtime responsive.
- Displays a progress bar.
- Uses a unique temporary file.
- Preserves a free-space reserve.
- Cleans up the file automatically.
- Handles `ENOSPC` and interrupted system calls.

note: The allocation itself is intentionally performed in one worker thread.  
Adding several allocation threads would usually not improve throughput and  
would make free-space handling and cleanup more complicated.  
The Tokio main task remains available to update the progress bar or handle other asynchronous work.  

<hr/>

to Rust..  

with
- `compile_error!` prevents compilation on non-Linux systems.
- No `unwrap()` or `expect()`.
- `CString` conversion errors are handled.
- Progress-channel failure becomes a real error.
- `spawn_blocking` join failures are propagated.
- `ProgressStyle` errors are propagated.
- Temporary-file deletion is explicitly checked with `close()`.
- The original allocation error is preserved if cleanup also fails.

<hr/>

to breaking to some modules with unit testings

```
zero-fill/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── cli/
    │   ├── mod.rs
    │   └── tests.rs
    ├── filesystem/
    │   ├── mod.rs
    │   └── tests.rs
    └── allocator/
        ├── mod.rs
        └── tests.rs
```

### cli

Responsible only for:

- Parsing command-line arguments.
- Parsing human-readable sizes such as `256M`, `2G`, and `1T`.
- Validating arguments.
- Producing an immutable configuration object.

Unit-test targets

### filesystem

Responsible only for Linux filesystem operations:

- Checking available space with `statvfs`.
- Creating the secure temporary file.
- Calling `fallocate`.
- Calling `fsync`.
- Removing or closing the temporary file.

### allocator

Responsible for the allocation algorithm:

- Applying the configured reserve.
- Applying the optional maximum size.
- Allocating one chunk at a time.
- Handling `EINTR`.
- Handling `ENOSPC` as a normal stopping condition.
- Sending progress events to the main task.

<hr/>

## Build and run

```
cargo test
cargo build --release
```

do not run from `/mnt/` (Windows folders), download the binary (unzip) and copy it to your profile (`~`),  
use `sudo chmod u+x ./zero-fill`, then just `sudo ./zero-fill`, do this after you clean-up your (virtual) machine.  


### Preview the operation without allocating:

```
sudo ./target/release/zero-fill \
    --path /var/tmp \
    --reserve 4G \
    --dry-run
```

### Allocate while preserving 4 GiB:

```
sudo ./target/release/zero-fill \
    --path /var/tmp \
    --reserve 4G \
    --chunk-size 256M
```

### Limit the allocation to 10 GiB:

```
sudo ./target/release/zero-fill \
    --path /var/tmp \
    --reserve 4G \
    --max-size 10G
```

### Keep the allocated file instead of deleting it:

```
sudo ./target/release/zero-fill \
    --path /var/tmp \
    --reserve 4G \
    --keep-file
```

The `--keep-file` option is intentionally explicit. Without it, the uniquely named temporary file is removed after `fsync()` succeeds.




<hr/>
<hr/>


### build

```
rustup update

cargo clean

rustup target add   x86_64-linux-android   i686-linux-android   aarch64-linux-android   armv7-linux-androideabi
cargo build  --release  --target   x86_64-linux-android
cargo build  --release  --target   i686-linux-android
cargo build  --release  --target   aarch64-linux-android
cargo build  --release  --target   armv7-linux-androideabi

#sudo apt-get update && sudo apt-get upgrade && sudo apt-get install --yes android-sdk-libsparse-utils apt-fast apt-transport-https aptitude aria2 asciidoc autoconf automake autopoint autotools-dev base-files bash bash-completion binutils binutils-aarch64-linux-gnu binutils-aarch64-linux-gnu-dbg binutils-alpha-linux-gnu binutils-alpha-linux-gnu-dbg binutils-arc-linux-gnu binutils-arc-linux-gnu-dbg binutils-arm-linux-gnueabi binutils-arm-linux-gnueabi-dbg binutils-arm-linux-gnueabihf binutils-arm-linux-gnueabihf-dbg binutils-arm-none-eabi binutils-avr binutils-bpf binutils-common binutils-dev binutils-djgpp binutils-doc binutils-for-build binutils-for-host binutils-h8300-hms binutils-hppa64-linux-gnu binutils-hppa64-linux-gnu-dbg binutils-hppa-linux-gnu binutils-hppa-linux-gnu-dbg binutils-i686-gnu binutils-i686-gnu-dbg binutils-i686-kfreebsd-gnu binutils-i686-kfreebsd-gnu-dbg binutils-i686-linux-gnu binutils-i686-linux-gnu-dbg binutils-ia64-linux-gnu binutils-ia64-linux-gnu-dbg binutils-loongarch64-linux-gnu binutils-loongarch64-linux-gnu-dbg binutils-m68hc1x binutils-m68k-linux-gnu binutils-m68k-linux-gnu-dbg binutils-mingw-w64 binutils-mingw-w64-i686 binutils-mingw-w64-x86-64 binutils-mips64-linux-gnuabi64 binutils-mips64-linux-gnuabi64-dbg binutils-mips64-linux-gnuabin32 binutils-mips64-linux-gnuabin32-dbg binutils-mips64el-linux-gnuabi64 binutils-mips64el-linux-gnuabi64-dbg binutils-mips64el-linux-gnuabin32 binutils-mips64el-linux-gnuabin32-dbg binutils-mips-linux-gnu binutils-mips-linux-gnu-dbg binutils-mipsel-linux-gnu binutils-mipsel-linux-gnu-dbg binutils-mipsisa32r6-linux-gnu binutils-mipsisa32r6-linux-gnu-dbg binutils-mipsisa32r6el-linux-gnu binutils-mipsisa32r6el-linux-gnu-dbg binutils-mipsisa64r6-linux-gnuabi64 binutils-mipsisa64r6-linux-gnuabi64-dbg binutils-mipsisa64r6-linux-gnuabin32 binutils-mipsisa64r6-linux-gnuabin32-dbg binutils-mipsisa64r6el-linux-gnuabi64 binutils-mipsisa64r6el-linux-gnuabi64-dbg binutils-mipsisa64r6el-linux-gnuabin32 binutils-mipsisa64r6el-linux-gnuabin32-dbg binutils-msp430 binutils-multiarch binutils-multiarch-dbg binutils-multiarch-dev binutils-or1k-elf binutils-powerpc64-linux-gnu binutils-powerpc64-linux-gnu-dbg binutils-powerpc64le-linux-gnu binutils-powerpc64le-linux-gnu-dbg binutils-powerpc-linux-gnu binutils-powerpc-linux-gnu-dbg binutils-riscv64-linux-gnu binutils-riscv64-linux-gnu-dbg binutils-riscv64-unknown-elf binutils-s390x-linux-gnu binutils-s390x-linux-gnu-dbg binutils-sh4-linux-gnu binutils-sh4-linux-gnu-dbg binutils-sh-elf binutils-source binutils-sparc64-linux-gnu binutils-sparc64-linux-gnu-dbg binutils-x86-64-gnu binutils-x86-64-gnu-dbg binutils-x86-64-kfreebsd-gnu binutils-x86-64-kfreebsd-gnu-dbg binutils-x86-64-linux-gnu binutils-x86-64-linux-gnu-dbg binutils-x86-64-linux-gnux32 binutils-x86-64-linux-gnux32-dbg binutils-xtensa-lx106 binutils-z80 binwalk bison bsdutils build-essential ca-certificates ccache checkinstall clang clisp-module-zlib cmake cmake-curses-gui cmake-data cmake-doc cmake-extras cmake-fedora cmake-format cmake-qt-gui cmake-vala coreutils curl dash debianutils devscripts dh-autoreconf diffutils docbook2x docbook-xsl docker.io dos2unix doxygen doxygen2man doxygen-awesome-css doxygen-doc doxygen-doxyparse doxygen-gui doxygen-latex dpkg-dev dpkg-dev-el elpa-dpkg-dev-el erlang-p1-zlib erofs-utils erofsfuse expat f2fs-tools findutils flex fuse2fs g++ g++-mingw-w64 g++-mingw-w64-i686 g++-mingw-w64-x86-64 gambas3-gb-compress-bzlib2 gambas3-gb-compress-zlib gcc gcc-aarch64-linux-gnu gcc-arm-linux-gnueabihf gcc-i686-linux-gnu gcc-mingw-w64 gcc-mingw-w64-i686 gcc-mingw-w64-x86-64 gcc-powerpc64-linux-gnu gcc-powerpc64le-linux-gnu gcc-powerpc-linux-gnu gcc-riscv64-linux-gnu gdb-mingw-w64 gedit gettext gfortran-mingw-w64 git glibc-doc glibc-doc-reference glibc-source glibc-tools gnat-mingw-w64 gnome-terminal gobjc-mingw-w64 gobjc++-mingw-w64 golang gperf grep gtk-doc-tools guile-lzlib guile-zlib gyp gzip hostname init intltool libassuan-mingw-w64-dev libattr1 libc6-dev libc6-dev-amd64-cross libc6-dev-amd64-i386-cross libc6-dev-amd64-x32-cross libc6-dev-arm64-cross libc6-dev-armhf-cross libc6-dev-i386 libc6-dev-powerpc-cross libc6-dev-powerpc-ppc64-cross libc6-dev-riscv64-cross libc-ares-dev libc++1 libc++abi1 libcompress-raw-zlib-perl libcppunit-dev libcurl4-openssl-dev libdpkg-dev libdwarf-dev libelf-dev libevent-2.1-7t64 libevent-core-2.1-7t64 libevent-dev libevent-distributor-perl libevent-execflow-perl libevent-extra-2.1-7t64 libevent-openssl-2.1-7t64 libevent-perl libevent-pthreads-2.1-7t64 libevent-rpc-perl libexpat1-dev libexpat-ocaml libexpat-ocaml-dev libffi-dev libfuse3-dev libgcrypt20-dev libgcrypt-mingw-w64-dev libghc-bzlib-dev libghc-bzlib-doc libghc-bzlib-prof libghc-zlib-bindings-dev libghc-zlib-bindings-doc libghc-zlib-bindings-prof libghc-zlib-dev libghc-zlib-doc libghc-zlib-prof libgmp-dev libgnatcoll-zlib3 libgnatcoll-zlib-dev libgnutls28-dev libgpg-error-mingw-w64-dev libguestfs-tools libjansson-dev libjzlib-java libksba-mingw-w64-dev libmpc-dev libmpfr-dev libncurses-dev libnpth-mingw-w64-dev libp11-kit-dev librte-compress-zlib24 libruby3.2 librust-async-compression-dev librust-expat-sys-dev librust-flate2-dev librust-gix-features-dev librust-grcov-dev librust-harfbuzz-sys-dev librust-khronos-egl-dev librust-libsodium-sys-dev librust-libsqlite3-sys-dev librust-libz-sys-dev librust-oxrocksdb-sys-dev librust-pkg-config-dev librust-pq-sys-dev librust-smithay-client-toolkit-dev librust-zip-dev librust-zstd-dev librust-zstd-safe-dev librust-zstd-sys-dev libsgmls-perl libsqlite3-dev libssh2-1-dev libssl-dev libtasn1-6-dev libtool libtool-bin libudev-dev libunistring-dev libxml2-dev libxml-sax-expat-incremental-perl libxml-sax-expatxs-perl libz-mingw-w64 libz-mingw-w64-dev lld llvm-dev login lua-expat lua-expat-dev lua-zlib lua-zlib-dev lzip m4 make mercurial mingw-w64 mingw-w64-common mingw-w64-i686-dev mingw-w64-tools mingw-w64-x86-64-dev musl musl-dev musl-tools nasm nautilus ncurses-base ncurses-bin nettle-dev ninja-build node-browserify-zlib npm openjdk-17-jdk openssh-server p7zip-full p11-kit-doc patch perl pkg-config pkgconf plocate pv python3 python3-colcon-pkg-config python3-docutils python3-jsonschema python3-mako python3-mesonpy python3-pip python3-requests python3-rstr python3-sphinx python-is-python3 r-bioc-zlibbioc ragel re2c ruby-pkg-config screen sed sgml-base sgml-base-doc sgml-data sgml-spell-checker sgmls-doc sgmlspl slang-expat software-properties-common subversion texinfo tree ubuntu-minimal ubuntu-wsl unzip util-linux uuid-dev wget win-iconv-mingw-w64-dev xmlto xsltproc yasm zlib1g-dev
rustup target add   x86_64-unknown-linux-gnu   aarch64-unknown-linux-gnu   x86_64-unknown-linux-musl   aarch64-unknown-linux-musl  powerpc-unknown-linux-gnu  powerpc64-unknown-linux-gnu  powerpc64le-unknown-linux-gnu
cargo build  --release  --target   x86_64-unknown-linux-gnu
cargo build  --release  --target   aarch64-unknown-linux-gnu
cargo build  --release  --target   x86_64-unknown-linux-musl
cargo build  --release  --target   aarch64-unknown-linux-musl
cargo build  --release  --target   powerpc-unknown-linux-gnu
cargo build  --release  --target   powerpc64-unknown-linux-gnu
cargo build  --release  --target   powerpc64le-unknown-linux-gnu
```

</details>

<br/>
<hr/>
<br/>