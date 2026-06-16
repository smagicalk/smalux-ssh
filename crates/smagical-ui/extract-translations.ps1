$ErrorActionPreference = "Stop"

$tool = Join-Path $PSScriptRoot "../../.tools/slint-tr-extractor/bin/slint-tr-extractor.exe"
$slintFile = Join-Path $PSScriptRoot "ui/main.slint"
$output = Join-Path $PSScriptRoot "messages.po"

& $tool `
  -o $output `
  --package-name "smagical-ui" `
  --package-version "0.1.0" `
  --join-existing `
  $slintFile
