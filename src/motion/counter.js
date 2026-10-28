// Translation and scale invariant full-body coordinates. Face/finger detail is
// excluded from matching, but all 33 landmarks are rendered by the viewer.
export function feature(points) {
  const ids = Array.from({length: 22}, (_, i) => i + 11);
  if (!points || ids.some(i => (points[i]?.visibility ?? 0) < .55)) return null;
  const center = {x:(points[23].x+points[24].x)/2,y:(points[23].y+points[24].y)/2};
  const shoulder = {x:(points[11].x+points[12].x)/2,y:(points[11].y+points[12].y)/2};
  const scale = Math.hypot(center.x-shoulder.x,center.y-shoulder.y);
  if (scale < .02) return null;
  return ids.flatMap(i => [(points[i].x-center.x)/scale,(points[i].y-center.y)/scale]);
}
export const distance = (a,b) => Math.sqrt(a.reduce((sum,x,i)=>sum+(x-b[i])**2,0)/a.length);
export function validatePattern(p) {
  if(p?.version!==1 || !Array.isArray(p.frames) || p.frames.length<5 || p.frames.length>150 || p.frames.some(f=>!Array.isArray(f)||f.length!==44||f.some(x=>!Number.isFinite(x)))) throw Error('Patrón inválido (5–150 cuadros de cuerpo completo).');
  if(Math.max(...p.frames.map(f=>distance(f,p.frames[0])))<.2) throw Error('El ejemplo no contiene suficiente movimiento.');
  return p.frames;
}
export class Counter {
  constructor(frames) { this.frames=frames; this.reset(); }
  reset() {this.count=0;this.index=0;this.armed=false;this.last=-Infinity;}
  missing() {this.index=0;this.armed=false;}
  update(f,time) {
    if(!f){this.missing();return this.count;}
    const p=this.frames;
    if(!this.armed){if(distance(f,p[0])<.28){this.armed=true;this.index=0;}return this.count;}
    // Only advance through nearby ordered samples, preventing endpoint-only counts.
    let best=this.index, error=distance(f,p[best]);
    for(let i=this.index+1;i<=Math.min(p.length-1,this.index+3);i++){const d=distance(f,p[i]);if(d<error){best=i;error=d;}}
    if(error<.32)this.index=best;
    if(this.index>=p.length-2 && distance(f,p.at(-1))<.28 && time-this.last>800){this.count++;this.last=time;this.missing();}
    return this.count;
  }
}
