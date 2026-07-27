[CmdletBinding()]
param(
  [Alias("w")]
  [string]$Worktree,

  [switch]$LaunchOnly,

  [switch]$InstallOnly,

  [switch]$Fresh,

  [switch]$List,

  [string]$VSCodiumPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if ($LaunchOnly -and $InstallOnly) {
  throw "-LaunchOnly and -InstallOnly cannot be used together."
}

if ($LaunchOnly -and $Fresh) {
  throw "-LaunchOnly cannot be combined with -Fresh because a fresh profile has no installed extension."
}

$scriptDir = $PSScriptRoot
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $scriptDir "..")).Path

function Invoke-GitCapture {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryPath,

    [Parameter(Mandatory = $true)]
    [string[]]$Arguments
  )

  $output = @(& git -C $RepositoryPath @Arguments 2>&1)
  if ($LASTEXITCODE -ne 0) {
    throw "git $($Arguments -join ' ') failed:`n$($output -join [Environment]::NewLine)"
  }

  return @($output | ForEach-Object { $_.ToString() })
}

function Invoke-NativeCommand {
  param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath,

    [Parameter(Mandatory = $true)]
    [string[]]$Arguments,

    [Parameter(Mandatory = $true)]
    [string]$WorkingDirectory
  )

  Push-Location -LiteralPath $WorkingDirectory
  try {
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
      throw "$FilePath $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
  }
  finally {
    Pop-Location
  }
}

function Normalize-Path {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Path
  )

  return [System.IO.Path]::GetFullPath($Path).TrimEnd([char[]]"\/")
}

function Get-GitCommonDirectory {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryPath
  )

  $commonDirectory = @(Invoke-GitCapture -RepositoryPath $RepositoryPath -Arguments @("rev-parse", "--git-common-dir"))[0]
  if (-not [System.IO.Path]::IsPathRooted($commonDirectory)) {
    $commonDirectory = Join-Path $RepositoryPath $commonDirectory
  }

  return Normalize-Path -Path $commonDirectory
}

function Get-OccWorktrees {
  param(
    [Parameter(Mandatory = $true)]
    [string]$RepositoryPath
  )

  $lines = @(Invoke-GitCapture -RepositoryPath $RepositoryPath -Arguments @("worktree", "list", "--porcelain"))
  $worktrees = @()
  $path = ""
  $branch = ""

  foreach ($line in ($lines + @(""))) {
    if ($line.StartsWith("worktree ")) {
      $path = $line.Substring("worktree ".Length)
    }
    elseif ($line.StartsWith("branch refs/heads/")) {
      $branch = $line.Substring("branch refs/heads/".Length)
    }
    elseif ($line.StartsWith("branch ")) {
      $branch = $line.Substring("branch ".Length)
    }
    elseif ([string]::IsNullOrWhiteSpace($line) -and -not [string]::IsNullOrWhiteSpace($path)) {
      $worktrees += [pscustomobject]@{
        Path = Normalize-Path -Path $path
        Branch = $branch
      }
      $path = ""
      $branch = ""
    }
  }

  return $worktrees
}

function Resolve-OccWorktree {
  param(
    [string]$Selector,

    [Parameter(Mandatory = $true)]
    [string]$RepositoryPath
  )

  $repoCommonDirectory = Get-GitCommonDirectory -RepositoryPath $RepositoryPath

  if ([string]::IsNullOrWhiteSpace($Selector)) {
    try {
      $currentTopLevel = @(Invoke-GitCapture -RepositoryPath (Get-Location).Path -Arguments @("rev-parse", "--show-toplevel"))[0]
      $currentCommonDirectory = Get-GitCommonDirectory -RepositoryPath $currentTopLevel
      if ($currentCommonDirectory -ieq $repoCommonDirectory) {
        return Normalize-Path -Path $currentTopLevel
      }
    }
    catch {
      # Fall back to the repository containing this script.
    }

    return Normalize-Path -Path $RepositoryPath
  }

  if (Test-Path -LiteralPath $Selector) {
    try {
      $candidate = @(Invoke-GitCapture -RepositoryPath $Selector -Arguments @("rev-parse", "--show-toplevel"))[0]
      if ((Get-GitCommonDirectory -RepositoryPath $candidate) -ieq $repoCommonDirectory) {
        return Normalize-Path -Path $candidate
      }
    }
    catch {
      # Continue with named worktree matching.
    }
  }

  $conventionalPath = Join-Path (Join-Path $RepositoryPath ".worktrees") $Selector
  if (Test-Path -LiteralPath $conventionalPath -PathType Container) {
    return Normalize-Path -Path $conventionalPath
  }

  $matches = @(
    Get-OccWorktrees -RepositoryPath $RepositoryPath |
      Where-Object {
        $_.Path -ieq $Selector -or
        (Split-Path -Leaf $_.Path) -ieq $Selector -or
        $_.Branch -ieq $Selector
      }
  )

  if ($matches.Count -eq 1) {
    return $matches[0].Path
  }

  if ($matches.Count -gt 1) {
    throw "Ambiguous worktree selector: $Selector"
  }

  throw "Unknown worktree: $Selector"
}

function Get-VSCodiumCommands {
  param(
    [string]$ExplicitPath
  )

  if (-not [string]::IsNullOrWhiteSpace($ExplicitPath)) {
    if (Test-Path -LiteralPath $ExplicitPath -PathType Leaf) {
      $resolvedPath = (Resolve-Path -LiteralPath $ExplicitPath).Path
    }
    else {
      $explicitCommand = Get-Command $ExplicitPath -ErrorAction SilentlyContinue | Select-Object -First 1
      if ($null -eq $explicitCommand) {
        throw "VSCodium executable not found: $ExplicitPath"
      }
      $resolvedPath = $explicitCommand.Source
    }

    if ([System.IO.Path]::GetExtension($resolvedPath) -ieq ".cmd") {
      $guiPath = Normalize-Path -Path (Join-Path (Split-Path -Parent $resolvedPath) "..\VSCodium.exe")
      if (-not (Test-Path -LiteralPath $guiPath -PathType Leaf)) {
        $guiPath = $resolvedPath
      }
      return [pscustomobject]@{ Cli = $resolvedPath; Gui = $guiPath }
    }

    $cliPath = Join-Path (Split-Path -Parent $resolvedPath) "bin\codium.cmd"
    if (-not (Test-Path -LiteralPath $cliPath -PathType Leaf)) {
      $cliPath = $resolvedPath
    }
    return [pscustomobject]@{ Cli = $cliPath; Gui = $resolvedPath }
  }

  if (-not [string]::IsNullOrWhiteSpace($env:OCC_VSCODIUM_EXECUTABLE)) {
    return Get-VSCodiumCommands -ExplicitPath $env:OCC_VSCODIUM_EXECUTABLE
  }

  $cliCommand = Get-Command "codium.cmd" -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($null -eq $cliCommand) {
    $cliCommand = Get-Command "codium" -ErrorAction SilentlyContinue | Select-Object -First 1
  }

  $guiCandidates = @(
    (Join-Path $env:ProgramFiles "VSCodium\VSCodium.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\VSCodium\VSCodium.exe"),
    (Join-Path ${env:ProgramFiles(x86)} "VSCodium\VSCodium.exe")
  )
  $guiPath = $guiCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1

  if ($null -ne $cliCommand -and -not [string]::IsNullOrWhiteSpace($guiPath)) {
    return [pscustomobject]@{ Cli = $cliCommand.Source; Gui = (Resolve-Path -LiteralPath $guiPath).Path }
  }

  if ($null -ne $cliCommand) {
    return Get-VSCodiumCommands -ExplicitPath $cliCommand.Source
  }

  if (-not [string]::IsNullOrWhiteSpace($guiPath)) {
    return Get-VSCodiumCommands -ExplicitPath $guiPath
  }

  return $null
}

function Assert-EditorCommands {
  param(
    [Parameter(Mandatory = $true)]
    [pscustomobject]$Commands
  )

  foreach ($path in @($Commands.Cli, $Commands.Gui)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
      throw "Editor command not found: $path"
    }
  }
}

function Get-EditorVersion {
  param(
    [Parameter(Mandatory = $true)]
    [pscustomobject]$Commands
  )

  $versionOutput = @(& $Commands.Cli "--version" 2>&1)
  if ($LASTEXITCODE -ne 0) {
    return $null
  }

  foreach ($line in $versionOutput) {
    if ($line.ToString() -match '^\s*(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)') {
      return $Matches[1]
    }
  }

  return $null
}

function Test-EditorCompatibility {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ResolverPath,

    [Parameter(Mandatory = $true)]
    [string]$ExtensionDirectory,

    [Parameter(Mandatory = $true)]
    [string]$Version
  )

  & node $ResolverPath "supports" $ExtensionDirectory $Version | Out-Null
  $status = $LASTEXITCODE
  if ($status -eq 0) {
    return $true
  }
  if ($status -eq 1) {
    return $false
  }

  throw "Failed to check editor compatibility with exit code $status."
}

function Get-ManagedVSCodeCommands {
  param(
    [Parameter(Mandatory = $true)]
    [string]$ResolverPath,

    [Parameter(Mandatory = $true)]
    [string]$ExtensionDirectory,

    [Parameter(Mandatory = $true)]
    [string]$CacheDirectory
  )

  $output = @(& node $ResolverPath "managed" $ExtensionDirectory $CacheDirectory 2>&1)
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to resolve managed VS Code:`n$($output -join [Environment]::NewLine)"
  }
  if ($output.Count -ne 3) {
    throw "Expected three lines from the managed VS Code resolver, received $($output.Count)."
  }

  return [pscustomobject]@{
    Version = $output[0].ToString()
    Gui = $output[1].ToString()
    Cli = $output[2].ToString()
  }
}

function ConvertTo-ProcessArgument {
  param(
    [Parameter(Mandatory = $true)]
    [string]$Value
  )

  if ($Value -notmatch '[\s"]') {
    return $Value
  }

  return '"' + $Value.Replace('"', '\"') + '"'
}

$knownWorktrees = @(Get-OccWorktrees -RepositoryPath $repoRoot)
if ($List) {
  foreach ($knownWorktree in $knownWorktrees) {
    $label = if ([string]::IsNullOrWhiteSpace($knownWorktree.Branch)) {
      Split-Path -Leaf $knownWorktree.Path
    }
    else {
      $knownWorktree.Branch
    }
    Write-Output ("{0,-24} {1}" -f $label, $knownWorktree.Path)
  }
  exit 0
}

$worktreePath = Resolve-OccWorktree -Selector $Worktree -RepositoryPath $repoRoot
$commonDirectory = Get-GitCommonDirectory -RepositoryPath $worktreePath
$worktreeRecord = $knownWorktrees | Where-Object { $_.Path -ieq $worktreePath } | Select-Object -First 1
$worktreeLabel = if ($null -ne $worktreeRecord -and -not [string]::IsNullOrWhiteSpace($worktreeRecord.Branch)) {
  $worktreeRecord.Branch
}
else {
  Split-Path -Leaf $worktreePath
}
$worktreeSlug = [regex]::Replace($worktreeLabel, "[^A-Za-z0-9._-]+", "-").Trim([char[]]"-")

$extensionDirectory = Join-Path $worktreePath "extension"
$manifestPath = Join-Path $extensionDirectory "package.json"
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
$vsixPath = Join-Path $extensionDirectory ("opencodecommit-{0}.vsix" -f $manifest.version)
$resolverPath = Join-Path $scriptDir "resolve-vscode-editor.mjs"
$testElectronManifest = Join-Path $extensionDirectory "node_modules\@vscode\test-electron\package.json"

if (-not $LaunchOnly -or -not (Test-Path -LiteralPath $testElectronManifest -PathType Leaf)) {
  Invoke-NativeCommand -FilePath "bun" -Arguments @("install", "--frozen-lockfile") -WorkingDirectory $extensionDirectory
}

if ($null -eq (Get-Command "node" -ErrorAction SilentlyContinue | Select-Object -First 1)) {
  throw "Node.js is required to select a compatible VS Code/VSCodium editor."
}

$engineOutput = @(& node $resolverPath "engine" $extensionDirectory 2>&1)
if ($LASTEXITCODE -ne 0 -or $engineOutput.Count -ne 1) {
  throw "Failed to read engines.vscode:`n$($engineOutput -join [Environment]::NewLine)"
}
$vscodeEngine = $engineOutput[0].ToString()

$vscodium = Get-VSCodiumCommands -ExplicitPath $VSCodiumPath
$editor = $null
if ($null -ne $vscodium) {
  Assert-EditorCommands -Commands $vscodium
  $vscodiumVersion = Get-EditorVersion -Commands $vscodium
  if (-not [string]::IsNullOrWhiteSpace($vscodiumVersion) -and
    (Test-EditorCompatibility -ResolverPath $resolverPath -ExtensionDirectory $extensionDirectory -Version $vscodiumVersion)) {
    $editor = [pscustomobject]@{
      Name = "VSCodium"
      Version = $vscodiumVersion
      StateName = "vscodium"
      Cli = $vscodium.Cli
      Gui = $vscodium.Gui
    }
  }
  elseif ([string]::IsNullOrWhiteSpace($vscodiumVersion)) {
    Write-Output "Could not determine the installed VSCodium version; using managed VS Code."
  }
  else {
    Write-Output "Installed VSCodium $vscodiumVersion does not satisfy engines.vscode $vscodeEngine; using managed VS Code."
  }
}
else {
  Write-Output "VSCodium was not found; using managed VS Code."
}

if ($null -eq $editor) {
  $managedVSCode = Get-ManagedVSCodeCommands `
    -ResolverPath $resolverPath `
    -ExtensionDirectory $extensionDirectory `
    -CacheDirectory (Join-Path $repoRoot ".vscode-test")
  Assert-EditorCommands -Commands $managedVSCode
  $editor = [pscustomobject]@{
    Name = "Visual Studio Code"
    Version = $managedVSCode.Version
    StateName = "vscode"
    Cli = $managedVSCode.Cli
    Gui = $managedVSCode.Gui
  }
}

$stateRoot = Join-Path (Join-Path (Join-Path $commonDirectory "dev") $editor.StateName) $worktreeSlug

if ($Fresh) {
  $sessionName = "session-{0}-{1}" -f (Get-Date).ToUniversalTime().ToString("yyyyMMdd-HHmmss"), $PID
  $stateRoot = Join-Path $stateRoot $sessionName
}

$userDataDirectory = Join-Path $stateRoot "user-data"
$extensionsDirectory = Join-Path $stateRoot "extensions"
New-Item -ItemType Directory -Force -Path $userDataDirectory, $extensionsDirectory | Out-Null

Write-Output "version: $($manifest.version)"
Write-Output "editor: $($editor.Name) $($editor.Version)"
Write-Output "worktree: $worktreePath"
Write-Output "user-data-dir: $userDataDirectory"
Write-Output "extensions-dir: $extensionsDirectory"
Write-Output "vsix: $vsixPath"
Write-Output "editor-cli: $($editor.Cli)"
Write-Output "editor-gui: $($editor.Gui)"

if (-not $LaunchOnly) {
  Invoke-NativeCommand -FilePath "bun" -Arguments @("run", "build:vsix") -WorkingDirectory $extensionDirectory
  Invoke-NativeCommand -FilePath "bunx" -Arguments @("@vscode/vsce", "package", "--out", $vsixPath) -WorkingDirectory $extensionDirectory
  Invoke-NativeCommand -FilePath $editor.Cli -Arguments @(
    "--user-data-dir", $userDataDirectory,
    "--extensions-dir", $extensionsDirectory,
    "--install-extension", $vsixPath,
    "--force"
  ) -WorkingDirectory $worktreePath

  $listArguments = @(
    "--user-data-dir", $userDataDirectory,
    "--extensions-dir", $extensionsDirectory,
    "--list-extensions",
    "--show-versions"
  )
  $installedExtensions = @(& $editor.Cli @listArguments 2>&1)
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to list extensions from the isolated editor profile:`n$($installedExtensions -join [Environment]::NewLine)"
  }

  $expectedExtension = "{0}.{1}@{2}" -f $manifest.publisher, $manifest.name, $manifest.version
  $installedExtension = $installedExtensions |
    Where-Object { $_.ToString().Trim() -ieq $expectedExtension } |
    Select-Object -First 1
  if ($null -eq $installedExtension) {
    throw "$($editor.Name) did not report $expectedExtension after installation.`nInstalled extensions:`n$($installedExtensions -join [Environment]::NewLine)"
  }
  Write-Output "verified-extension: $($installedExtension.ToString().Trim())"
}

if (-not $InstallOnly) {
  $launchArguments = @(
    "--new-window",
    "--user-data-dir", $userDataDirectory,
    "--extensions-dir", $extensionsDirectory,
    $worktreePath
  ) | ForEach-Object { ConvertTo-ProcessArgument -Value $_ }

  Start-Process -FilePath $editor.Gui -ArgumentList $launchArguments -WorkingDirectory $worktreePath | Out-Null
}
