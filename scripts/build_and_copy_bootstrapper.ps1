# PowerShell script to build the bootstrapper and copy it to src-tauri/binaries with target triple in the name

# Define paths
$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$projectRoot = Resolve-Path "$root\.."
$manifestPath = "$projectRoot\bootstrapper\Cargo.toml"

# Build the bootstrapper
cargo build --release --manifest-path $manifestPath

# Get the target triple from rustc
$triple = (rustc -vV | Select-String 'host:').ToString().Split(':')[1].Trim()

$bootstrapperExe = "$projectRoot\bootstrapper\target\release\bootstrapper.exe"
$destDir = "$projectRoot\src-tauri\binaries"
$destExe = "$destDir\bootstrapper-$triple.exe"

# Create destination directory if it doesn't exist
if (!(Test-Path $destDir)) {
    New-Item -ItemType Directory -Path $destDir | Out-Null
}

# Copy and rename the executable
Copy-Item $bootstrapperExe $destExe -Force
Write-Host "Bootstrapper built and copied to $destExe"

