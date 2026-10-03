// UTMT CLI script: exports nine-slice settings (sprites that have them enabled) to $RK_OUT/nineslice.json
using System;
using System.IO;
using System.Text;
EnsureDataLoaded();
var sb = new StringBuilder("{\n");
bool first = true;
foreach (var s in Data.Sprites)
{
    var ns = s?.V3NineSlice;
    if (ns == null || !ns.Enabled) continue;
    if (!first) sb.Append(",\n");
    first = false;
    sb.Append($"\"{s.Name.Content}\":[{ns.Left},{ns.Top},{ns.Right},{ns.Bottom}]");
}
sb.Append("\n}\n");
File.WriteAllText(Path.Combine(Environment.GetEnvironmentVariable("RK_OUT"), "nineslice.json"), sb.ToString());
Console.WriteLine("nineslice done");
