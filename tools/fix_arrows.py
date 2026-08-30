# -*- coding: utf-8 -*-
# Swap LEFT/RIGHT arrow glyphs (they were reversed: eab6=chevron-right).
p = 'src/ui/icons.rs'
s = open(p, encoding='utf-8').read()
LEFT = chr(0xEAB6)
RIGHT = chr(0xEAB8)
s = s.replace('pub const LEFT_ARROW: &str = "' + LEFT + '"; // chevron-left',
              'pub const LEFT_ARROW: &str = "' + RIGHT + '"; // chevron-left')
s = s.replace('pub const RIGHT_ARROW: &str = "' + RIGHT + '"; // chevron-right',
              'pub const RIGHT_ARROW: &str = "' + LEFT + '"; // chevron-right')
open(p, 'w', encoding='utf-8').write(s)
import re
for m in re.finditer(r'pub const (LEFT_ARROW|RIGHT_ARROW): &str = "(.)"', s):
    print(m.group(1), hex(ord(m.group(2))))
