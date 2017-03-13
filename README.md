## build

on mac os, install xquartz, and then
```
    export LIBRARY_PATH=/opt/X11/lib
    cargo build --release
```

## Design
use DSL for rule to filter windows and other stuff

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
- [ ] cut off long name display 
