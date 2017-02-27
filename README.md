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
or(attr(map_state=Viewable), geom(and(x>2, w>100)))
```

id wildcard matching
```
and(id=0x10000??, attr(override_redirect=true))
```

actions
```
attr(map_state=Viewable): filter;
not(attr(map_state=Viewable)): pin;
```

## TODO

- [ ] do idle update
- [ ] use DSL to specify filter rule
- [ ] highlight diffs across events
- [ ] pin windows (highlight some windows) 
