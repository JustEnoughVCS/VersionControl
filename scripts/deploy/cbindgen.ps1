$originalLocation = Get-Location
Set-Location (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))

if (-not (Get-Command cbindgen -ErrorAction SilentlyContinue)) {
    cargo install cbindgen
}

if (Test-Path ffi/.temp/jvlib.h) {
    Remove-Item ffi/.temp/jvlib.h
}
$env:RUSTUP_TOOLCHAIN = "nightly"
cbindgen `
    --config cbindgen.toml `
    ffi `
    --output ffi/.temp/jvlib.h `
    --quiet

Set-Location $originalLocation
