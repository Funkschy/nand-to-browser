# LaTeX-Vorlage für Abschlussarbeiten

Dies ist die Vorlage für Bachelor- oder Masterarbeiten
des Lehrstuhls für Softwaretechnik und Programmiersprachen.

## Dateien

Sie finden in diesem Repository folgende Dateien:

- `thesis.tex`:
  Dies ist die Hauptdatei der Arbeit,
  welche zur finalen PDF kompiliert.
  In dieser Datei geben Sie Ihren Namen und die Daten zu Ihrer Arbeit an.
- `contents.tex`:
  Diese Datei enthält den textlichen Inhalt der Vorlage.
  Im Rahmen Ihrer schriftlichen Ausarbeitung ist es sinnvoll,
  einzelne Abschnitte in ihre eigenen Dateien auszulagern.
- `abstract.tex`:
  Enthält die maximal einseitige Zusammenfassung Ihrer Arbeit.
- `appendix.tex`:
  Ähnlich zur `contents.tex`;
  enthält sämtlichen eventuellen Anhang zu Ihrer Arbeit.
- `references.bib`:
  Enthält die BibTeX-Einträge Ihrer verwendeten Quellen.
- `Makefile`:
  Für `make`-Befehle; siehe unten.
- `.bibtoolrsc`:
  Formatierungsvorlage für `make bibtool`.
- `hhuthesis.cls`:
  Dokumentklasse für die LaTeX-Vorlage - Unverändert lassen!

## Makefile

Der Vorlage liegt ein `Makefile` bei.
Über das Terminal können Sie somit folgende Kommandos aufrufen:

```bash
# Baut die PDF
make

# Löscht alle generierten LaTeX-Dateien (inklusive PDF)
make clean

# Sortiert die BibTeX-Einträge und sorgt für einheitliche Formatierung
make bibtool

# Baut die PDF aus den Sources stetig neu, updated die Anzeige in Ihrem PDF-Betrachter
make watch
```

Das Makefile nutzt hierbei `latexmk`,
welches entsprechend auf Ihrem System zur Verfügung stehen muss.

## Schwarzweißdruck

Die Arbeit ist standartmäßig mit farbigem HHU-Logo und farbigen pgfplots
konfiguriert.
Da im Druck jede farbige Seite zusätzliche Kosten verursacht,
können Sie sich auch für einen Schwarzweißdruck entscheiden.
Nutzen Sie hierfür in der Präambel der `thesix.tex`
den folgenden Befehl:

```latex
\blackwhiteprint
```

Sie werden in der `thesis.tex` bereits eine entsprechende Stelle vorfinden,
an welcher der Befehl auskommentiert steht.
Es genügt dort das Kommentarzeichen zu entfernen.

### Farben

Falls der Farbdruck gewählt wird,
sind in der Dokumentklasse die Farben
`hhublue`, `hhudarkblue`, `hhuiceblue`, `hhucyan`, `hhugreen`, `hhuorange`
und `hhured` vordefiniert.
Diese werden ebenfalls standartmäßig als Graphfarben genutzt,
weswegen es sich empfiehlt auf diese Farben aus Konsistenzgründen
zurückzugreifen.

Die Farben sind entsprechend der Corporate Design Guidelines der HHU definiert.
