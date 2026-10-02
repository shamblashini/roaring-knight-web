// Exports precise collision masks: name -> {w,h,masks:[base64 bitpacked rows]}
using System;
using System.IO;
using System.Text;
EnsureDataLoaded();
string outDir = Environment.GetEnvironmentVariable("RK_OUT");
var sb = new StringBuilder("{\n");
bool first = true;
foreach (var spr in Data.Sprites)
{
    if (spr == null || spr.CollisionMasks.Count == 0) continue;
    var dims = spr.CalculateMaskDimensions(Data);
    if (!first) sb.Append(",\n");
    first = false;
    sb.Append($"\"{spr.Name.Content}\":{{\"w\":{dims.Item1},\"h\":{dims.Item2},\"masks\":[");
    for (int i = 0; i < spr.CollisionMasks.Count; i++)
    {
        if (i > 0) sb.Append(",");
        sb.Append("\"" + Convert.ToBase64String(spr.CollisionMasks[i].Data) + "\"");
    }
    sb.Append("]}");
}
sb.Append("\n}\n");
File.WriteAllText(Path.Combine(outDir, "masks.json"), sb.ToString());
Console.WriteLine("masks done");
