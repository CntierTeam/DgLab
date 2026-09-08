# Install DgLab binary + Codex skill from GitHub Releases (Windows PowerShell).
# 仅学习用
[CmdletBinding()]
param(
    [switch]$BinOnly,
    [switch]$SkillOnly,
    [string]$Version = $(if ($env:DGLAB_VERSION) { $env:DGLAB_VERSION } else { "latest" }),
    [string]$Repo = $(if ($env:DGLAB_REPO) { $env:DGLAB_REPO } else { "CntierTeam/DgLab" }),
    [string]$Prefix = $(if ($env:DGLAB_PREFIX) { $env:DGLAB_PREFIX } else { (Join-Path $env:USERPROFILE ".local") }),
    [string]$CodexHome = $(if ($env:CODEX_HOME) { $env:CODEX_HOME } else { (Join-Path $env:USERPROFILE ".codex") }),
    [switch]$Force,
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$BinDir = Join-Path $Prefix "bin"
$SkillDst = Join-Path $CodexHome "skills\dglab"
$BinPath = Join-Path $BinDir "dglab.exe"
$Api = "https://api.github.com/repos/$Repo"
$InstallBin = -not $SkillOnly
$InstallSkill = -not $BinOnly

function Get-ReleaseJson {
    $headers = @{ "Accept" = "application/vnd.github+json"; "User-Agent" = "dglab-install" }
    if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $($env:GITHUB_TOKEN)" }
    $url = if ($Version -eq "latest") { "$Api/releases/latest" } else { "$Api/releases/tags/$Version" }
    return Invoke-RestMethod -Uri $url -Headers $headers
}

function Download-Asset([string]$Url, [string]$OutFile) {
    $headers = @{ "User-Agent" = "dglab-install" }
    if ($env:GITHUB_TOKEN) { $headers["Authorization"] = "Bearer $($env:GITHUB_TOKEN)" }
    Invoke-WebRequest -Uri $Url -Headers $headers -OutFile $OutFile
}

if ($Uninstall) {
    if (Test-Path $BinPath) { Remove-Item -Force $BinPath; Write-Host "removed $BinPath" }
    if (Test-Path $SkillDst) { Remove-Item -Recurse -Force $SkillDst; Write-Host "removed $SkillDst" }
    exit 0
}

$release = Get-ReleaseJson
Write-Host "release: $Repo@$($release.tag_name)"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("dglab-install-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    if ($InstallBin) {
        $assetName = "dglab-x86_64-pc-windows-msvc.tar.gz"
        $asset = $release.assets | Where-Object { $_.name -eq $assetName } | Select-Object -First 1
        if (-not $asset) { throw "asset not found: $assetName" }
        $archive = Join-Path $tmp $assetName
        Write-Host "downloading $assetName"
        Download-Asset $asset.browser_download_url $archive
        tar -C $tmp -xzf $archive
        $exe = Get-ChildItem -Path $tmp -Recurse -Filter dglab.exe | Select-Object -First 1
        if (-not $exe) { throw "dglab.exe missing in archive" }
        New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
        if ((Test-Path $BinPath) -and -not $Force) { throw "already exists: $BinPath (use -Force)" }
        Copy-Item -Force $exe.FullName $BinPath
        Write-Host "binary: $BinPath"
    }

    if ($InstallSkill) {
        $assetName = "dglab-skill.tar.gz"
        $asset = $release.assets | Where-Object { $_.name -eq $assetName } | Select-Object -First 1
        if (-not $asset) { throw "asset not found: $assetName" }
        $archive = Join-Path $tmp $assetName
        Write-Host "downloading $assetName"
        Download-Asset $asset.browser_download_url $archive
        $skillTmp = Join-Path $tmp "skill"
        New-Item -ItemType Directory -Path $skillTmp | Out-Null
        tar -C $skillTmp -xzf $archive
        $src = Join-Path $skillTmp "dglab"
        if (-not (Test-Path (Join-Path $src "SKILL.md"))) { throw "skill SKILL.md missing" }
        New-Item -ItemType Directory -Force -Path (Split-Path $SkillDst) | Out-Null
        if ((Test-Path $SkillDst) -and -not $Force) { throw "already exists: $SkillDst (use -Force)" }
        if (Test-Path $SkillDst) { Remove-Item -Recurse -Force $SkillDst }
        Copy-Item -Recurse $src $SkillDst
        Write-Host "skill: $SkillDst"
    }
}
finally {
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Done."
if ($InstallBin) {
    if (-not ($env:PATH -split ";" | Where-Object { $_ -eq $BinDir })) {
        Write-Host "note: add to PATH → $BinDir"
    }
    Write-Host "try: dglab --mock --headless"
}
if ($InstallSkill) {
    Write-Host "Codex skill: `$dglab (restart Codex / new session if already running)"
}
