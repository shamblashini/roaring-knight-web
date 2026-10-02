#!/usr/bin/env python3
"""Strip GML `if` blocks whose condition mentions content irrelevant to the knight fight (reading aid only)."""
import re, sys
BAD = re.compile(r'rouxls|tenna|shadowman|elnina|lanino|susiezilla|chefs|rhythm|shootout|room_board|gameshow|zapper|board|mike|ramb|lightemup|sharpshoot|minigame|global\.chapter == [124]|chapter < 3|chapter == 1|chapter == 2|noelle|berdly|spamton|queen|ch1|ch2|global\.plot|snowgrave|jevil|giga|flag\[915\]|weird|obj_dw_|krisRS|tutorial|global\.lang == "ja"|langopt|kris_rhythm|cyber|doom_|mettaton|sans|tasque|werewire|poppup|spade|clover|lancer|king|ruddin|hathy|virovirokun|ambyu|swatch|maus|lancer|alt_gameover|obj_rudebuster_|redbuster|pacify_counter|lightworld')
def prune(src):
    lines = src.split("\n")
    out = []
    i = 0
    while i < len(lines):
        l = lines[i]
        s = l.strip()
        m = re.match(r'(else )?if \((.*)\)$', s)
        if m and BAD.search(m.group(2)) and not re.search(r'== false|== 0\b|^!|\(!', m.group(2)) and i + 1 < len(lines) and lines[i + 1].strip() == "{":
            depth = 0
            j = i + 1
            while j < len(lines):
                depth += lines[j].count("{") - lines[j].count("}")
                if depth == 0:
                    break
                j += 1
            out.append(l.replace(s, f"/* pruned: {s[:60]} */"))
            i = j + 1
            continue
        out.append(l)
        i += 1
    return "\n".join(out)
txt = open(sys.argv[1]).read()
txt = re.sub(r'\nenum e__VW[\s\S]*$', '', txt)
print(prune(prune(txt)))
