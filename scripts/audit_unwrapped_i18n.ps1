# 扫描所有 .slint 文件中未包裹 @tr(...) 的中文字符串

$files = Get-ChildItem -Path "crates/smagical-ui/ui" -Filter "*.slint" -Recurse
$unwrapped = @()

foreach ($f in $files) {
    $lines = Get-Content $f.FullName -Encoding UTF8
    for ($i = 0; $i -lt $lines.Length; $i++) {
        $line = $lines[$i]
        $trimmed = $line.Trim()
        # 跳过纯注释行
        if ($trimmed.StartsWith("//") -or $trimmed.StartsWith("/*") -or $trimmed.StartsWith("*") -or $trimmed.StartsWith("///")) {
            continue
        }
        # 检查是否包含双引号中的中文字符
        if ($line -match '"[^"]*[\p{IsCJKUnifiedIdeographs}]+[^"]*"') {
            # 如果整行不包含 @tr(
            if ($line -notmatch '@tr\(') {
                $unwrapped += [PSCustomObject]@{
                    File = $f.Name
                    RelPath = $f.FullName.Replace("F:\code\rust\smalux-ssh\", "")
                    Line = $i + 1
                    Code = $trimmed
                }
            }
        }
    }
}

Write-Host "=========================================="
Write-Host "Total unwrapped Chinese string lines: $($unwrapped.Count)"
Write-Host "=========================================="

$byFile = $unwrapped | Group-Object RelPath | Sort-Object Count -Descending
foreach ($g in $byFile) {
    Write-Host "`n----------------------------------------"
    Write-Host "$($g.Name) ($($g.Count) lines):"
    Write-Host "----------------------------------------"
    foreach ($item in $g.Group) {
        Write-Host "  L$($item.Line): $($item.Code)"
    }
}

