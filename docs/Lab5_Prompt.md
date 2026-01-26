Der folgende Prompt ist nur zu Dokumentationszwecken aufgeführt:

```
Ich habe dieses Fragen mit den Antworten   Übersetze alles auf Englisch und schreibe alles ausführlich 
Mach es schöner und verwende auch Funnktionsausschnitte


1. How do lifetimes ensure that the returned BorrowedEntry values cannot outlive the
store?

Der Iterator leiht sich Daten direkt aus dem Store. Solange diese Leihe existiert muss der Store noch da sein (Im Scope). Sobald der Store weg ist dürfen diese geliehenen Einträge nicht mehr benutzt werden also verhindert, dass man auf Speicher zugreift der schon freigegeben ist.



2. What happens if you mutate the store (i.e. call put/insert) while iterating? Should this
be allowed?

Weil wenn man durch den Store läuft, darf man ihn nur readen, nicht verändern.
Bei Einfügen oder Ändern könnte man den Speicher verschieben dann würden die aktuellen Einträge plötzlich auf falsche Stellen zeigen. Deshlab ist das Verändern während des Durchlaufes nicht erlaubt.



3. The StoreIter type iterates over immutable borrows of entries, which
point into the underlying Vec<u8> buffer of the store. Could you write a StoreIterMut
that returns mutably-borrowed entries? What would happen if you change the data type
of a key or value this way? Think about the memory representation of your serialized
entries.





Die Daten liegen als rohe Bytesfolge hintereinander im Speicher. Wenn man einen Wert ändert und er anders wird, passt alles danach nicht mehr. Der RawHeader passt dann nicht mehr mit dem Ursprungs Inhalt. Der Iterator würde falsche Daten lesen oder abstürzen. Also die Änderungen müssen über extra Funktionen passieren, nicht direkt beim Durchlaufnen.
Wenn Sichergestellt wird, dass die längt immer gleich bleibt und der typ nicht geändert werden kann, dann wäre es theoretisch möglich.


Erwähne bei der ersten Frage, dass es der Store::iter(&self) ist.
```