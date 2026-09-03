$lines = Get-Content "crates/smagical-ui/translations/template.pot" -Encoding UTF8
$dict = [System.Collections.Generic.HashSet[string]]::new()

foreach ($line in $lines) {
    if ($line.StartsWith('msgid "')) {
        $id = $line.Substring(7, $line.Length - 8)
        if ($id.Length -gt 0 -and $id -match '[\p{IsCJKUnifiedIdeographs}]') {
            $dict.Add($id) | Out-Null
        }
    }
}

$list = [System.Collections.Generic.List[string]]::new($dict)
$list.Sort()
[System.IO.File]::WriteAllLines("scripts/pot_unique_chinese.txt", $list, [System.Text.Encoding]::UTF8)
Write-Host "Extracted $($list.Count) unique Chinese msgid entries to scripts/pot_unique_chinese.txt"
