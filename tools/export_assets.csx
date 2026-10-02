// UTMT CLI script: exports sprites (padded frames + metadata), sounds, fonts and object metadata
// into the directory named by env var RK_OUT. Run against a COPY of data.win, never the install.
using System;
using System.IO;
using System.Linq;
using System.Text;
using System.Collections.Generic;
using UndertaleModLib.Util;

EnsureDataLoaded();
string outDir = Environment.GetEnvironmentVariable("RK_OUT");
Directory.CreateDirectory(outDir);
string sprDir = Path.Combine(outDir, "sprites");
string sndDir = Path.Combine(outDir, "sounds");
string fntDir = Path.Combine(outDir, "fonts");
Directory.CreateDirectory(sprDir); Directory.CreateDirectory(sndDir); Directory.CreateDirectory(fntDir);

string J(string s) => "\"" + (s ?? "").Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\n", "\\n") + "\"";

// ---- sprites
var sb = new StringBuilder("{\n");
bool first = true;
using (var worker = new TextureWorker())
{
    foreach (var spr in Data.Sprites)
    {
        if (spr == null || spr.Textures.Count == 0) continue;
        if (!first) sb.Append(",\n");
        first = false;
        sb.Append($"{J(spr.Name.Content)}:{{\"w\":{spr.Width},\"h\":{spr.Height},\"ox\":{spr.OriginX},\"oy\":{spr.OriginY}," +
                  $"\"bl\":{spr.MarginLeft},\"br\":{spr.MarginRight},\"bt\":{spr.MarginTop},\"bb\":{spr.MarginBottom}," +
                  $"\"frames\":{spr.Textures.Count},\"sepmasks\":{(int)spr.SepMasks},\"bboxmode\":{spr.BBoxMode}," +
                  $"\"speed\":{spr.GMS2PlaybackSpeed.ToString(System.Globalization.CultureInfo.InvariantCulture)},\"speedtype\":{(int)spr.GMS2PlaybackSpeedType}}}");
        for (int i = 0; i < spr.Textures.Count; i++)
        {
            var t = spr.Textures[i]?.Texture;
            if (t == null) continue;
            worker.ExportAsPNG(t, Path.Combine(sprDir, $"{spr.Name.Content}_{i}.png"), null, true);
        }
    }
}
sb.Append("\n}\n");
File.WriteAllText(Path.Combine(outDir, "sprites.json"), sb.ToString());

// ---- fonts
sb = new StringBuilder("{\n"); first = true;
using (var worker = new TextureWorker())
{
    foreach (var f in Data.Fonts)
    {
        if (f?.Texture == null) continue;
        worker.ExportAsPNG(f.Texture, Path.Combine(fntDir, $"{f.Name.Content}.png"));
        if (!first) sb.Append(",\n");
        first = false;
        sb.Append($"{J(f.Name.Content)}:{{\"size\":{f.EmSize.ToString(System.Globalization.CultureInfo.InvariantCulture)},\"ascender\":{f.AscenderOffset},\"glyphs\":[");
        sb.Append(string.Join(",", f.Glyphs.Select(g => $"[{g.Character},{g.SourceX},{g.SourceY},{g.SourceWidth},{g.SourceHeight},{g.Shift},{g.Offset}]")));
        sb.Append("]}");
    }
}
sb.Append("\n}\n");
File.WriteAllText(Path.Combine(outDir, "fonts.json"), sb.ToString());

// ---- sounds (embedded, incl. audio groups)
var groups = new Dictionary<int, IList<UndertaleEmbeddedAudio>>();
IList<UndertaleEmbeddedAudio> Group(int id)
{
    if (groups.ContainsKey(id)) return groups[id];
    string p = Path.Combine(Path.GetDirectoryName(FilePath), $"audiogroup{id}.dat");
    IList<UndertaleEmbeddedAudio> r = null;
    if (File.Exists(p))
        using (var s = new FileStream(p, FileMode.Open, FileAccess.Read))
            r = UndertaleIO.Read(s, (w, _) => { }).EmbeddedAudio;
    groups[id] = r;
    return r;
}
sb = new StringBuilder("{\n"); first = true;
foreach (var snd in Data.Sounds)
{
    if (snd == null) continue;
    byte[] bytes = null;
    if (snd.GroupID == Data.GetBuiltinSoundGroupID()) bytes = snd.AudioFile?.Data;
    else { var g = Group(snd.GroupID); if (g != null && snd.AudioID >= 0 && snd.AudioID < g.Count) bytes = g[snd.AudioID].Data; }
    string file = "";
    if (bytes != null)
    {
        string ext = (bytes.Length > 4 && bytes[0] == 'O' && bytes[1] == 'g') ? ".ogg" : ".wav";
        file = snd.Name.Content + ext;
        File.WriteAllBytes(Path.Combine(sndDir, file), bytes);
    }
    if (!first) sb.Append(",\n");
    first = false;
    sb.Append($"{J(snd.Name.Content)}:{{\"file\":{J(file)},\"ext\":{J(snd.File?.Content)},\"vol\":{snd.Volume.ToString(System.Globalization.CultureInfo.InvariantCulture)},\"pitch\":{snd.Pitch.ToString(System.Globalization.CultureInfo.InvariantCulture)}}}");
}
sb.Append("\n}\n");
File.WriteAllText(Path.Combine(outDir, "sounds.json"), sb.ToString());

// ---- objects
sb = new StringBuilder("{\n"); first = true;
foreach (var o in Data.GameObjects)
{
    if (o == null) continue;
    if (!first) sb.Append(",\n");
    first = false;
    sb.Append($"{J(o.Name.Content)}:{{\"sprite\":{J(o.Sprite?.Name?.Content)},\"mask\":{J(o.TextureMaskId?.Name?.Content)},\"parent\":{J(o.ParentId?.Name?.Content)},\"depth\":{o.Depth},\"visible\":{(o.Visible ? 1 : 0)}}}");
}
sb.Append("\n}\n");
File.WriteAllText(Path.Combine(outDir, "objects.json"), sb.ToString());
Console.WriteLine("export done");
