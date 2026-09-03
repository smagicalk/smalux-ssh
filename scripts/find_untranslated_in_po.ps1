$lines = Get-Content "crates/smagical-ui/translations/en/LC_MESSAGES/smagical-ui.po" -Encoding UTF8
$untrans = [System.Collections.Generic.List[string]]::new()
$currentId = ""

for ($i = 0; $i -lt $lines.Length; $i++) {
    $l = $lines[$i]
    if ($l.StartsWith('msgid "')) {
        $currentId = $l.Substring(7, $l.Length - 8)
    }
    if ($l.StartsWith('msgstr "')) {
        $str = $l.Substring(8, $l.Length - 9)
        if ($str -eq $currentId -and $str -match '[\p{IsCJKUnifiedIdeographs}]') {
            $untrans.Add($str)
        }
    }
}

[System.IO.File]::WriteAllLines("scripts/untranslated_remaining.txt", $untrans, [System.Text.Encoding]::UTF8)
Write-Host "Written $($untrans.Count) untranslated entries to scripts/untranslated_remaining.txt"
