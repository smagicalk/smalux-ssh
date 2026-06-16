$ErrorActionPreference = "Stop"

$slintFile = Join-Path $PSScriptRoot "ui/main.slint"
$output = Join-Path $PSScriptRoot "messages.po"

& slint-tr-extractor `
  -o $output `
  --package-name "smagical-ui" `
  --package-version "0.1.0" `
  --join-existing `
  $slintFile
