# Builds and installs an isolated package; never opens the normal app profile.
# Run from PowerShell: & .\scripts\verify-native.ps1
param([string]$Repository = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repository = [IO.Path]::GetFullPath($Repository)
Set-Location -LiteralPath $repository
$env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ';' + $env:PATH
# The existing MSVC toolchain can be present even when rustup shims are absent.
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    $toolchainBin = Join-Path $env:USERPROFILE '.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin'
    if (Test-Path -LiteralPath (Join-Path $toolchainBin 'cargo.exe')) { $env:PATH = $toolchainBin + ';' + $env:PATH }
}
# Keep release compilation within the tested machine's bounded memory budget.
$env:CARGO_BUILD_JOBS = '1'
$runId = [Guid]::NewGuid().ToString('N')
$runRoot = Join-Path $repository ".artifacts\native-$runId"
$installed = Join-Path $runRoot 'installed'
$profile = Join-Path $runRoot 'profile'
$product = "Ascension Verification $($runId.Substring(0, 8))"
New-Item -ItemType Directory -Path $runRoot | Out-Null
Write-Output "Isolated verification run: native-$runId"
$config = Join-Path $runRoot 'tauri.conf.json'
$configuration = @{
    productName = $product
    identifier = "io.ascension.verification.r$runId"
    app = @{ appDirectoriesOverride = $profile }
} | ConvertTo-Json -Depth 5
[IO.File]::WriteAllText($config, $configuration, [Text.UTF8Encoding]::new($false))

# PowerShell 5 can turn native stderr (including harmless Tauri Info lines)
# into terminating errors when the caller captures streams. Use exit codes.
try {
    $ErrorActionPreference = 'Continue'
    & npm.cmd run tauri -- build --config $config -- --locked --offline
    $buildExit = $LASTEXITCODE
} finally { $ErrorActionPreference = 'Stop' }
if ($buildExit -ne 0) { throw 'Isolated package build failed.' }
$package = Join-Path $repository "src-tauri\target\release\bundle\nsis\$($product)_0.1.0_x64-setup.exe"
try {
    $installer = Start-Process -FilePath $package -ArgumentList @('/S', "/D=$installed") -WindowStyle Hidden -Wait -PassThru
    if ($installer.ExitCode -ne 0) { throw "Installer exited $($installer.ExitCode)." }
    $executable = Join-Path $installed 'ascension.exe'
    if (!(Test-Path -LiteralPath $executable)) { throw 'Installed executable is missing.' }
    Write-Output 'Isolated package built and installed; beginning native lifecycle verification.'
    try {
        $ErrorActionPreference = 'Continue'
        & node scripts/native-regression.mjs $runRoot
        $nativeExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = 'Stop' }
    if ($nativeExit -ne 0) { throw 'Installed application regression failed; see the isolated run directory.' }
    @{
        installer_exit = $installer.ExitCode
        installer_sha256 = (Get-FileHash -LiteralPath $package -Algorithm SHA256).Hash
        installed_sha256 = (Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash
        installer_bytes = (Get-Item -LiteralPath $package).Length
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $runRoot 'package-result.json') -Encoding UTF8
} finally {
    # Resolve and check this invocation's destination before executing its uninstaller.
    $resolvedInstall = [IO.Path]::GetFullPath($installed)
    if (!$resolvedInstall.StartsWith([IO.Path]::GetFullPath($runRoot) + '\', [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Refusing cleanup outside the isolated verification directory.'
    }
    $uninstaller = Join-Path $resolvedInstall 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller) {
        $removed = Start-Process -FilePath $uninstaller -ArgumentList '/S' -WindowStyle Hidden -Wait -PassThru
        if ($removed.ExitCode -ne 0) { throw "Verification uninstall exited $($removed.ExitCode)." }
    }
}
Write-Output "Native verification passed. Synthetic evidence remains in .artifacts/native-$runId."
