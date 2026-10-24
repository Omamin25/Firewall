<#
.SYNOPSIS
    Generates vendor/npcap/lib/wpcap.lib from the installed Npcap runtime DLL.

.DESCRIPTION
    The `pcap` crate links against `wpcap.lib`, which ships only in the Npcap SDK.
    The Npcap *runtime* installer does not include it. This script rebuilds an
    equivalent import library from the exports of the already-installed
    `wpcap.dll` using the MSVC toolchain (dumpbin + lib), so the project builds
    without downloading the SDK.

    If you have the official Npcap SDK installed, you do not need this: add the
    SDK's Lib\x64 folder to the LIB environment variable instead. The linker
    searches LIB before the vendored path, so the SDK always takes precedence.

.NOTES
    Requires Visual Studio (or MSVC Build Tools) with the C++ toolchain.
    Re-run this if Npcap is upgraded and its export list changes.
#>
[CmdletBinding()]
param(
    [string]$DllPath,
    [string]$OutputDir = (Join-Path $PSScriptRoot '..\vendor\npcap\lib')
)

$ErrorActionPreference = 'Stop'

function Find-WpcapDll {
    if ($DllPath) {
        if (-not (Test-Path -LiteralPath $DllPath)) { throw "wpcap.dll not found at '$DllPath'." }
        return (Resolve-Path -LiteralPath $DllPath).Path
    }
    $candidates = @(
        "$env:SystemRoot\System32\Npcap\wpcap.dll",
        "$env:SystemRoot\System32\wpcap.dll"
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate) { return (Resolve-Path -LiteralPath $candidate).Path }
    }
    throw 'Could not find wpcap.dll. Install Npcap (WinPcap API-compatible mode) or pass -DllPath.'
}

function Find-MsvcTool {
    param([Parameter(Mandatory)][string]$Name)

    $onPath = Get-Command $Name -ErrorAction SilentlyContinue
    if ($onPath) { return $onPath.Source }

    $roots = @(
        "${env:ProgramFiles}\Microsoft Visual Studio",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio"
    ) | Where-Object { Test-Path -LiteralPath $_ }

    $tool = $roots |
        ForEach-Object { Get-ChildItem -LiteralPath $_ -Recurse -Filter $Name -ErrorAction SilentlyContinue } |
        Where-Object { $_.FullName -match 'Hostx64\\x64' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1

    if (-not $tool) {
        throw "Could not locate $Name. Install the MSVC C++ build tools, or run this from a Developer PowerShell."
    }
    return $tool.FullName
}

$dll = Find-WpcapDll
$dumpbin = Find-MsvcTool -Name 'dumpbin.exe'
$lib = Find-MsvcTool -Name 'lib.exe'

Write-Host "wpcap.dll : $dll"
Write-Host "dumpbin   : $dumpbin"
Write-Host "lib       : $lib"

# Parse the `ordinal hint RVA name` table out of dumpbin's export listing.
$exportLines = & $dumpbin /EXPORTS $dll
if ($LASTEXITCODE -ne 0) { throw "dumpbin failed with exit code $LASTEXITCODE." }

$symbols = [System.Collections.Generic.List[string]]::new()
$inTable = $false
foreach ($line in $exportLines) {
    if ($line -match '^\s*ordinal\s+hint\s+RVA\s+name') { $inTable = $true; continue }
    if (-not $inTable) { continue }
    if ($line -match '^\s*\d+\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]{8}\s+(\S+)') { $symbols.Add($Matches[1]) }
}

if ($symbols.Count -eq 0) { throw 'No exported symbols parsed from wpcap.dll.' }
Write-Host "Parsed $($symbols.Count) exported symbols."

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
$resolvedOut = (Resolve-Path -LiteralPath $OutputDir).Path
$defPath = Join-Path $resolvedOut 'wpcap.def'
$libPath = Join-Path $resolvedOut 'wpcap.lib'

$def = [System.Collections.Generic.List[string]]::new()
$def.Add('LIBRARY wpcap.dll')
$def.Add('EXPORTS')
foreach ($symbol in $symbols) { $def.Add("    $symbol") }
Set-Content -LiteralPath $defPath -Value $def -Encoding ASCII

& $lib "/def:$defPath" /machine:x64 "/out:$libPath" | Out-Null
if ($LASTEXITCODE -ne 0) { throw "lib.exe failed with exit code $LASTEXITCODE." }

Write-Host "Generated $libPath" -ForegroundColor Green
