[CmdletBinding()]
param()

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-CommandPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name
    )

    $command = Get-Command -Name $Name -ErrorAction SilentlyContinue
    if ($null -eq $command) {
        return $null
    }

    return $command.Source
}

function Test-Administrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-InstalledVersion {
    param(
        [Parameter(Mandatory = $true)]
        [AllowEmptyString()]
        [string]$CommandPath,

        [Parameter(Mandatory = $true)]
        [string]$Arguments
    )

    if ([string]::IsNullOrWhiteSpace($CommandPath)) {
        return $null
    }

    $version = & $CommandPath $Arguments 2>$null | Select-Object -First 1
    if ([string]::IsNullOrWhiteSpace($version)) {
        return 'installed'
    }

    return $version.Trim()
}

function Test-VersionAtLeast {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Version,

        [Parameter(Mandatory = $true)]
        [Version]$Minimum
    )

    try {
        return ([Version]($Version.TrimStart('v'))) -ge $Minimum
    }
    catch {
        return $false
    }
}

function Get-NpcapEvidence {
    $evidence = [System.Collections.Generic.List[string]]::new()
    $service = Get-Service -Name 'npcap' -ErrorAction SilentlyContinue
    if ($null -ne $service) {
        $evidence.Add("service=$($service.Status)")
    }

    foreach ($path in @(
        'C:\Program Files\Npcap',
        'C:\Program Files (x86)\Npcap',
        'C:\Windows\System32\Npcap\wpcap.dll',
        'C:\Windows\System32\drivers\npcap.sys'
    )) {
        if (Test-Path -LiteralPath $path) {
            $evidence.Add($path)
        }
    }

    foreach ($key in @(
        'HKLM:\SOFTWARE\Npcap',
        'HKLM:\SOFTWARE\WOW6432Node\Npcap'
    )) {
        if (Test-Path -LiteralPath $key) {
            $evidence.Add($key)
        }
    }

    return $evidence
}

function Get-WinDivertEvidence {
    $evidence = [System.Collections.Generic.List[string]]::new()
    $service = Get-CimInstance Win32_SystemDriver -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '^WinDivert' -or $_.DisplayName -match '^WinDivert' } |
        Select-Object -First 1
    if ($null -ne $service) {
        $evidence.Add("driver=$($service.State)")
    }

    foreach ($path in @(
        'C:\Windows\System32\WinDivert.dll',
        'C:\Windows\System32\WinDivert64.sys',
        'C:\Windows\SysWOW64\WinDivert.dll',
        'C:\Windows\SysWOW64\WinDivert32.sys',
        (Join-Path $PSScriptRoot '..\src-tauri\WinDivert.dll'),
        (Join-Path $PSScriptRoot '..\src-tauri\WinDivert64.sys')
    )) {
        if (Test-Path -LiteralPath $path) {
            $evidence.Add((Resolve-Path -LiteralPath $path).Path)
        }
    }

    $workspaceVendorPath = Join-Path $PSScriptRoot '..\vendor\windivert'
    if (Test-Path -LiteralPath $workspaceVendorPath) {
        Get-ChildItem -LiteralPath $workspaceVendorPath -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -in @('WinDivert.dll', 'WinDivert32.dll', 'WinDivert64.sys', 'WinDivert32.sys') } |
            ForEach-Object { $evidence.Add($_.FullName) }
    }

    return $evidence
}

function Get-MsvcEvidence {
    $evidence = [System.Collections.Generic.List[string]]::new()
    foreach ($root in @(
        'C:\Program Files\Microsoft Visual Studio',
        'C:\Program Files (x86)\Microsoft Visual Studio'
    )) {
        if (Test-Path -LiteralPath $root) {
            Get-ChildItem -LiteralPath $root -Recurse -File -ErrorAction SilentlyContinue -Include cl.exe,link.exe |
                ForEach-Object { $evidence.Add($_.FullName) }
        }
    }

    return $evidence
}

$results = [System.Collections.Generic.List[object]]::new()

$nodePath = Get-CommandPath -Name 'node.exe'
$nodeVersion = Get-InstalledVersion -CommandPath $nodePath -Arguments '--version'
$results.Add([pscustomobject]@{
        Name = 'Node.js 20+'
        Passed = ($null -ne $nodeVersion -and (Test-VersionAtLeast -Version $nodeVersion -Minimum ([Version]'20.0')))
        Detail = if ($null -eq $nodeVersion) { 'not found' } else { $nodeVersion }
    })

$rustPath = Get-CommandPath -Name 'rustc.exe'
$cargoPath = Get-CommandPath -Name 'cargo.exe'
$rustVersion = Get-InstalledVersion -CommandPath $rustPath -Arguments '--version'
$cargoVersion = Get-InstalledVersion -CommandPath $cargoPath -Arguments '--version'
$rustReady = $null -ne $rustVersion -and $null -ne $cargoVersion
$results.Add([pscustomobject]@{
        Name = 'Rust + Cargo (MSVC toolchain)'
        Passed = $rustReady
        Detail = if ($rustReady) { "$rustVersion; $cargoVersion" } else { 'rustc/cargo not found' }
    })

$msvcCommands = @(
    (Get-CommandPath -Name 'cl.exe'),
    (Get-CommandPath -Name 'link.exe')
) | Where-Object { $null -ne $_ }
$msvcCommands = @($msvcCommands)
$msvcEvidence = @(Get-MsvcEvidence)
$msvcReady = ($msvcCommands.Count -ge 2) -or (
    ($msvcEvidence | Where-Object { $_ -match '\\cl\.exe$' }).Count -gt 0 -and
    ($msvcEvidence | Where-Object { $_ -match '\\link\.exe$' }).Count -gt 0
)
$results.Add([pscustomobject]@{
        Name = 'MSVC linker'
        Passed = $msvcReady
        Detail = if ($msvcCommands.Count -ge 2) { 'cl.exe and link.exe available in PATH' } elseif ($msvcReady) { 'MSVC binaries installed; use Developer PowerShell' } else { 'cl.exe/link.exe not found' }
    })

$npcapEvidence = @(Get-NpcapEvidence)
$results.Add([pscustomobject]@{
        Name = 'Npcap (WinPcap-compatible)'
        Passed = $npcapEvidence.Count -gt 0
        Detail = if ($npcapEvidence.Count -gt 0) { $npcapEvidence -join ', ' } else { 'not found' }
    })

$windivertEvidence = @(Get-WinDivertEvidence)
$results.Add([pscustomobject]@{
        Name = 'WinDivert 2.x'
        Passed = $windivertEvidence.Count -gt 0
        Detail = if ($windivertEvidence.Count -gt 0) { $windivertEvidence -join ', ' } else { 'not found' }
    })

$isAdministrator = Test-Administrator
$results.Add([pscustomobject]@{
        Name = 'Administrator shell'
        Passed = $isAdministrator
        Detail = if ($isAdministrator) { 'elevated' } else { 'not elevated' }
    })

Write-Host 'Cognton Phase 0 prerequisite verification'
Write-Host ''
$results | Format-Table -AutoSize

$failed = @($results | Where-Object { -not $_.Passed })
if ($failed.Count -gt 0) {
    Write-Host "Phase 0 is incomplete: $($failed.Count) prerequisite(s) failed."
    exit 1
}

Write-Host 'Phase 0 prerequisites are ready.'
exit 0
