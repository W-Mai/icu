# icu
Image Converter Ultra

# Architecture

```text
       ╔═══════════════╗                       
       ║               ║                       
       ║               ║                       
┌ ─ ─ ─ ─ ─ ─ ┐        ║                       
  ┌ ─ ─ ─ ─ ┐          ║                       
│  EnDecoder  │        ▼                       
  └ ─ ─ ─ ─ ┘   ┌ ─ ─ ─ ─ ─ ─ ┐                
│┌───────────┐│   ┌ ─ ─ ─ ─ ┐                  
 │    PNG    │  │   MidData   │                
│└───────────┘│   └ ─ ─ ─ ─ ┘                  
 ┌───────────┐  │┌───────────┐│                
││   JPEG    ││  │   ARGB    │                 
 └───────────┘  │└───────────┘│ ╔-------------╗
│┌───────────┐│  ┌───────────┐  ║   ICU_LIB   ║
 │    SVG    │  ││   PATH    ││ ╚-------------╝
│└───────────┘│  └───────────┘                 
 ┌───────────┐  │┌── ─── ─── ┐│                
││ LVGL BIN  ││     CUSTOM   │                 
 └───────────┘  │└── ─── ─── ┘│                
│┌── ─── ─── ┐│  ─ ─ ─ ─ ─ ─ ─                 
    CUSTOM   │         ║                       
│└── ─── ─── ┘│        ║                       
 ─ ─ ─ ─ ─ ─ ─         ║                       
       ▲               ║                       
       ║               ║                       
       ╚═══════════════╝                       
```
