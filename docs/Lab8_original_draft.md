## Hinweis zur Entstehung dieser Ausarbeitung

Diese Ausarbeitung war ursprünglich nicht dafür gedacht, veröffentlicht oder hochgeladen zu werden. Das Hochladen erfolgte lediglich auf Wunsch des Professors.

Da eine Veröffentlichung nicht geplant war, wurde keine besondere Sorgfalt auf Rechtschreibung, Grammatik oder stilistische Ausarbeitung gelegt. Der Fokus lag ausschließlich auf dem Inhalt und der korrekten Beantwortung der Fragestellungen.

Der ursprüngliche Prompt zur Erstellung der Datei [Lab8_answers_(compaction).md](Lab8_answers_(compaction).md) ist inzwischen nicht mehr vorhanden. Der inhaltliche Ursprung der Ausarbeitung basiert auf den im Rahmen der Übung ausgearbeiteten Fragestellungen und Antworten.

Die ursprüngliche Ausarbeitung:

```
1. What is the time complexity of the compaction step?

Die Zeitkomplexität ist O(n + S). Weil Kopieren einfach das Teuerste ist was hier passiert. Ist es ist in der Realität am dominantesten

Wobei n = Anzahl der Keys & s = Größe aller gültigen Daten.

O(n + S), weil wenn die Summe aller kopierten Bytes S ist, kostet das O(S)
und wenn der Index n Keys enthält, dauert dieser Teil linear O(n)



2. When is a good time to run the compaction step?


1. wenn viele Einträge gelöscht oder überschrieben wurden
2. wenn viel toter Speicher vorhanden ist
3. bei Wartungsarbeiten ( beim Start oder beim Beenden der Anwendung)
4. wenn der Speicher zu voll wird kann man einfach manuell aufräumen ohne dass die Anwendung ausgebremst wird


Damit der User selbst entscheiden kann wann der Aufwand  anfällt ist die Kompaktierung bewusst manuell gehalten
```