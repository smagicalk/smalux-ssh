$ErrorActionPreference = "Stop"

$uiDir = Join-Path $PSScriptRoot "ui"
$slintFiles = Get-ChildItem -Path $uiDir -Filter "*.slint" -Recurse | ForEach-Object { $_.FullName }
$output = Join-Path $PSScriptRoot "messages.po"

& slint-tr-extractor `
  -o $output `
  --package-name "smagical-ui" `
  --package-version "0.1.0" `
  --join-existing `
  $slintFiles
