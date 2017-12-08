## build

on mac os, install xquartz, and then
```
    export LIBRARY_PATH=/opt/X11/lib
    cargo build --release
```

## Design
use DSL for rule to filter windows and other stuff

the grammar can be display by 
`cargo run -- --show-grammar`


filtering windows
```
any(attrs.map_state=Viewable, all(geom.x>2, geom.width>100))
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
- [x] pin windows (highlight some windows everlasting) 
- [ ] ignore some events
- [ ] (de)serialize rules from/into disk 
- [ ] event timestamp?
- [ ] rule databases (based on serialization)
- [x] cut off long name display 
- [ ] change rules on the fly (so I can change the set of monitored windows without restart)
- [ ] xrandr events may affect definition of visible
- [ ] on macOS, client id should be child of queried window
- [ ] query_tree is heavy, need to cache window stack like mutter does
- [ ] expand grammar to support window properties
- [ ] expand grammar to support conditional expression
- [ ] add gui support
- [ ] add log level and detail management
- [ ] on-the-fly rule injection/removal (via socket?)


## ideas
- better event tracing, add tracepoint dynamically, listen to any property/attrs change 
