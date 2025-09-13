@echo off
REM MediathekViewWeb CLI - Example Usage Script
REM Run this script to see various example searches

echo MediathekViewWeb CLI Examples
echo ============================
echo.

echo 1. List available channels:
echo mwb channels
echo.
mwb channels
echo.
echo Press any key to continue...
pause >nul

echo.
echo 2. Search for recent Tatort episodes:
echo mwb search "#Tatort" --size 5
echo.
mwb search "#Tatort" --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 3. Find documentaries longer than 30 minutes:
echo mwb search "Dokumentation" --min-duration 30 --size 5
echo.
mwb search "Dokumentation" --min-duration 30 --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 4. Search ARD news content excluding weather (regex):
echo mwb search "!ARD Nachrichten" --exclude "\bWetter\b|Wettervorhersage" --size 5
echo.
mwb search "!ARD Nachrichten" --exclude "\bWetter\b|Wettervorhersage" --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 5. Find documentaries with include regex filter:
echo mwb search "Dokumentation" --include "Klima|Umwelt|Science" --size 5
echo.
mwb search "Dokumentation" --include "Klima|Umwelt|Science" --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 6. Combine include and exclude regex filters:
echo mwb search "#Nachrichten" --include "Politik|Wirtschaft" --exclude "Sport|Wetter" --size 5
echo.
mwb search "#Nachrichten" --include "Politik|Wirtschaft" --exclude "Sport|Wetter" --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 7. Find content with duration filtering:
echo mwb search "Tatort" --min-duration 80 --max-duration 90 --size 5
echo.
mwb search "Tatort" --min-duration 80 --max-duration 90 --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 8. Export ZDF content to CSV:
echo mwb search "!ZDF" --size 10 --format csv ^> zdf_content.csv
echo.
mwb search "!ZDF" --size 10 --format csv > zdf_content.csv
echo Content exported to zdf_content.csv
echo.
echo Press any key to continue...
pause >nul

echo.
echo 9. Find long-form content (movies and documentaries):
echo mwb search --min-duration 90 --size 5 --sort-by duration --sort-order desc
echo.
mwb search --min-duration 90 --size 5 --sort-by duration --sort-order desc
echo.
echo Press any key to continue...
pause >nul

echo.
echo 10. JSON output for scripting:
echo mwb search "!Arte" --size 3 --format json
echo.
mwb search "!Arte" --size 3 --format json
echo.
echo Press any key to continue...
pause >nul

echo.
echo 11. Create XSPF playlist and launch VLC (medium quality default):
echo mwb search "Tatort >80" --vlc --size 5
echo.
mwb search "Tatort >80" --vlc --size 5
echo.
echo Press any key to continue...
pause >nul

echo.
echo 11b. Create XSPF playlist and launch VLC with low quality:
echo mwb search "Tatort >80" --vlc=l --size 3
echo.
mwb search "Tatort >80" --vlc=l --size 3
echo.
echo Press any key to continue...
pause >nul

echo.
echo 11c. Create XSPF playlist and launch VLC with HD quality:
echo mwb search "Tatort >80" -v=h --size 3
echo.
mwb search "Tatort >80" -v=h --size 3
echo.
echo Press any key to continue...
pause >nul

echo.
echo 12. Save XSPF playlist to file (without launching VLC):
echo mwb search "Dokumentation >60" --format xspf --xspf-file --size 10
echo.
mwb search "Dokumentation >60" --format xspf --xspf-file --size 10
echo.
echo Press any key to continue...
pause >nul

echo.
echo 13. XSPF output to console for piping:
echo mwb search "Natur" --format xspf --size 5 ^> nature_playlist.xspf
echo.
mwb search "Natur" --format xspf --size 5 > nature_playlist.xspf
echo XSPF playlist saved to nature_playlist.xspf
echo.

echo.
echo Examples completed! Try creating your own searches using:
echo   Selectors: !channel #theme +title *description
echo   Duration: --min-duration X --max-duration Y (in minutes)
echo   Regex Exclusion: --exclude "pattern1" "pattern2"
echo   Regex Inclusion: --include "pattern1" "pattern2"
echo   Formats: --format table/json/csv/xspf
echo   Regex Examples: "word1|word2", "\bexact\b", "word.*", "[Tt]atort"
echo   Combined: mwb search "Dokumentation" --include "Klima|Natur" --min-duration 30
echo   VLC XSPF: mwb search "Tatort >80" --vlc (medium quality, default)
echo   VLC Quality: mwb search "Tatort" -v=l (low), -v=m (medium), -v=h (HD)
echo   XSPF File: mwb search "query" --format xspf --xspf-file
echo.
echo Press any key to exit...
pause >nul