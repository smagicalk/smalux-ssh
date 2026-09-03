# 国际化语言包提取脚本 (Extract i18n Translations)
# 扫描 crates/smagical-ui/ui 目录下所有 .slint 界面文件并提取 @tr 字符串

$ErrorActionPreference = "Stop"

$UiDir = "crates\smagical-ui\ui"
$TranslationsDir = "crates\smagical-ui\translations\en\LC_MESSAGES"
$PotFile = "crates\smagical-ui\translations\template.pot"
$GenScript = "scripts\generate_en_translations.ps1"

if (-not (Test-Path $TranslationsDir)) {
    New-Item -ItemType Directory -Force -Path $TranslationsDir | Out-Null
}

# 收集所有 .slint 文件
$SlintFiles = Get-ChildItem -Path $UiDir -Filter "*.slint" -Recurse | ForEach-Object { $_.FullName }

Write-Host "Found $($SlintFiles.Count) Slint files. Running slint-tr-extractor..."

# 提取到 template.pot
& slint-tr-extractor -o $PotFile --package-name "smalux-ssh" --package-version "0.1.0" $SlintFiles

Write-Host "Successfully generated translation template: $PotFile"

# 同步生成并更新英文翻译文件
if (Test-Path $GenScript) {
    & pwsh -File $GenScript
}
