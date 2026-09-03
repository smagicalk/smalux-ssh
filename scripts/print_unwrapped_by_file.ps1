param([string]$FilePath)

$lines = Get-Content $FilePath -Encoding UTF8
for ($i = 0; $i -lt $lines.Length; $i++) {
    $line = $lines[$i]
    $trimmed = $line.Trim()
    if ($trimmed.StartsWith("//") -or $trimmed.StartsWith("/*") -or $trimmed.StartsWith("*") -or $trimmed.StartsWith("///")) {
        continue
    }
    if ($line -match '"[^"]*[\p{IsCJKUnifiedIdeographs}]+[^"]*"') {
        if ($line -notmatch '@tr\(') {
            Write-Host ("L{0}: {1}" -f ($i + 1), $trimmed)
        }
    }
}
