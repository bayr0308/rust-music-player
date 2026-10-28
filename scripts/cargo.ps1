param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArgs)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$localCargo = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path -LiteralPath $localCargo) {
    $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:PATH = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:PATH
    $gnuTools = Join-Path $env:RUSTUP_HOME 'toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
    if (Test-Path -LiteralPath $gnuTools) { $env:PATH = $gnuTools + ';' + $env:PATH }
    $mingwTools = Join-Path $projectRoot '.tools\llvm-mingw-20261020-msvcrt-x86_64\bin'
    if (Test-Path -LiteralPath $mingwTools) { $env:PATH = $mingwTools + ';' + $env:PATH }
    if (Test-Path -LiteralPath $mingwTools) {
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $mingwTools 'x86_64-w64-mingw32-clang.exe'
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = '-C link-self-contained=yes'
    }
    & $localCargo @CargoArgs
} else {
    & cargo @CargoArgs
}
exit $LASTEXITCODE
