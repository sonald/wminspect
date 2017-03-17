## build

on mac os, install xquartz, and then
```
    export LIBRARY_PATH=/opt/X11/lib
    cargo build --release
```

## Design
use DSL for rule to filter windows and other stuff

the grammar is basically like this:
```
 top -> ( item ( ';' item )* )?
 item -> cond ( ':' action)? 
 cond -> pred op VAL
     | ANY '(' cond (',' cond )* ')'
     | ALL '(' cond (',' cond )* ')'
     | NOT '(' cond ')'
 pred -> ID ('.' ID)*
 op -> '=' | '>' | '<' | '>=' | '<=' | '<>'
 action -> 'filter' | 'pin'
 ID -> STRING_LIT
 VAL -> STRING_LIT
```

filtering windows
```
any(attrs.map_state=Viewable, all(geom.x>2, geom.w>100))
```

id wildcard matching
```
all(id=0x10000??, name=*mutter*, attrs.override_redirect=true)
```

actions
```
attrs.map_state=Viewable: filter;
not(attrs.map_state=Viewable): pin;
```

## TODO

- [x] do idle update
- [x] use DSL to specify filter rule (partially)
- [x] highlight diffs across events
- [ ] pin windows (highlight some windows everlasting) 
- [ ] ignore some events
- [ ] (de)serialize rules from/into disk 
- [ ] event timestamp?
- [ ] rule databases (based on serialization)
- [x] cut off long name display 
- [ ] change rules on the fly (so I can change the set of monitored windows without restart)
