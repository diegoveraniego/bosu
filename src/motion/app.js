import {feature,validatePattern,Counter} from './counter.js';
const $=id=>document.getElementById(id), video=$('video'), canvas=$('overlay'), ctx=canvas.getContext('2d');
let engine, connections, stream, recorder, sourceUrl, downloadUrl, begin=null, end=null, frames=null, counter=null, counting=false, busy=false, lastTime=-1;
const say=s=>$('status').textContent=s;
function failure(e){say(e.message||String(e));}
function controls(){const local=!!video.src && !stream && Number.isFinite(video.duration);for(const id of ['seek','prev','next','start','end'])$(id).disabled=!local||busy;$('train').disabled=!local||busy||begin===null||end===null;$('record').disabled=!stream||busy||recorder?.state==='recording';$('camera').disabled=!engine||busy;}
function resetMarks(){begin=end=null;$('marks').textContent='Sin marcas';}
function stopCamera(){if(recorder?.state==='recording')recorder.stop();stream?.getTracks().forEach(t=>t.stop());stream=null;video.srcObject=null;controls();}
function setVideo(blob){stopCamera();if(sourceUrl)URL.revokeObjectURL(sourceUrl);sourceUrl=URL.createObjectURL(blob);video.src=sourceUrl;video.muted=false;resetMarks();counter?.missing();}
function draw(){if(!engine||video.readyState<2)return null;const result=engine.detect(video);const points=result.landmarks[0];canvas.width=video.videoWidth;canvas.height=video.videoHeight;ctx.clearRect(0,0,canvas.width,canvas.height);if(points){ctx.strokeStyle='#85ffb7';ctx.fillStyle='#fff';ctx.lineWidth=3;for(const {start,end} of connections){const a=points[start],b=points[end];if(a.visibility<.55||b.visibility<.55)continue;ctx.beginPath();ctx.moveTo(a.x*canvas.width,a.y*canvas.height);ctx.lineTo(b.x*canvas.width,b.y*canvas.height);ctx.stroke();}for(const p of points){if(p.visibility<.55)continue;ctx.beginPath();ctx.arc(p.x*canvas.width,p.y*canvas.height,3,0,Math.PI*2);ctx.fill();}}return feature(points);}
$('load').onclick=async()=>{ $('load').disabled=true;say('Descargando motor y modelo…');try{const {PoseLandmarker,FilesetResolver}=await import('https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.32/vision_bundle.mjs');const files=await FilesetResolver.forVisionTasks('https://cdn.jsdelivr.net/npm/@mediapipe/tasks-vision@0.10.32/wasm');engine=await PoseLandmarker.createFromOptions(files,{baseOptions:{modelAssetPath:'https://storage.googleapis.com/mediapipe-models/pose_landmarker/pose_landmarker_lite/float16/1/pose_landmarker_lite.task'},runningMode:'IMAGE',numPoses:1});connections=PoseLandmarker.POSE_CONNECTIONS;$('file').disabled=false;controls();say('Motor listo. Abre un video o activa la cámara.');}catch(e){failure(e);$('load').disabled=false;}};
$('file').onchange=()=>{if($('file').files[0])setVideo($('file').files[0]);};
$('camera').onclick=async()=>{try{stopCamera();stream=await navigator.mediaDevices.getUserMedia({video:{width:640,height:480},audio:false});video.removeAttribute('src');video.srcObject=stream;video.muted=true;await video.play();resetMarks();controls();say('Cámara activa. Puedes grabar el ejemplo o contar con un patrón existente.');}catch(e){stopCamera();failure(e);}};
$('record').onclick=()=>{try{const chunks=[];recorder=new MediaRecorder(stream);recorder.ondataavailable=e=>{if(e.data.size)chunks.push(e.data);};recorder.onstop=()=>{const blob=new Blob(chunks,{type:recorder.mimeType});if(downloadUrl)URL.revokeObjectURL(downloadUrl);downloadUrl=URL.createObjectURL(blob);$('download').href=downloadUrl;$('download').download='bosu-ejemplo.webm';$('download').hidden=false;setVideo(blob);$('stop').disabled=true;say('Grabación lista. Revisa y marca una repetición completa.');};recorder.start();$('stop').disabled=false;controls();say('Grabando. Pulsa Detener al terminar la repetición.');}catch(e){failure(e);}};
$('stop').onclick=()=>{if(recorder?.state==='recording')recorder.stop();};$('off').onclick=()=>{stopCamera();say('Cámara apagada.');};
video.onloadedmetadata=()=>{
  // MediaRecorder WebM files may omit duration metadata. Seeking to the end
  // lets the browser determine their actual duration before frame review.
  if(video.duration===Infinity && !stream){
    video.addEventListener('seeked',()=>{video.currentTime=0;controls();},{once:true});
    video.currentTime=1e10;
  }
  $('seek').max=Number.isFinite(video.duration)?video.duration:0;controls();
};
video.ondurationchange=()=>{$('seek').max=Number.isFinite(video.duration)?video.duration:0;controls();};
video.onseeked=()=>{if(!busy){counter?.missing();try{draw();}catch(e){failure(e);}}};
video.onerror=()=>say('No se puede reproducir este video. Prueba un archivo MP4 o WebM compatible.');
function seek(t){video.pause();video.currentTime=Math.max(0,Math.min(video.duration,t));}
$('seek').oninput=()=>seek(Number($('seek').value));$('prev').onclick=()=>seek(video.currentTime-.2);$('next').onclick=()=>seek(video.currentTime+.2);
function marks(){ $('marks').textContent=`Inicio: ${begin?.toFixed(1)??'—'} s · Fin: ${end?.toFixed(1)??'—'} s`;controls();}
$('start').onclick=()=>{begin=video.currentTime;marks();};$('end').onclick=()=>{end=video.currentTime;marks();};
function seekAsync(time){return new Promise((resolve,reject)=>{if(Math.abs(video.currentTime-time)<.001){resolve();return;}const timeout=setTimeout(()=>{video.removeEventListener('seeked',done);reject(Error('No se pudo revisar el cuadro.'));},5000);function done(){clearTimeout(timeout);resolve();}video.addEventListener('seeked',done,{once:true});video.currentTime=time;});}
function usePattern(p){frames=validatePattern(p);counter=new Counter(frames);counting=false;$('counting').disabled=false;$('counting').textContent='Iniciar conteo';$('export').disabled=false;$('count').textContent='0 reps';$('phase').textContent='Patrón listo';}
$('train').onclick=async()=>{if(end-begin<.8||end-begin>30){say('Selecciona entre 0,8 y 30 segundos, de inicio a fin de una repetición completa.');return;}busy=true;counting=false;video.pause();controls();say('Analizando el ejemplo a 5 FPS…');const saved=video.currentTime;try{const samples=[];for(let t=begin;t<=end+.001;t+=.2){await seekAsync(Math.min(t,video.duration));const f=draw();if(!f)throw Error('El cuerpo no se ve completo en el ejemplo. Repite la toma con más luz y sin ocultar extremidades.');samples.push(f);}usePattern({version:1,frames:samples});say('Patrón listo. Reproduce otro video o usa la cámara e inicia el conteo.');}catch(e){failure(e);}finally{await seekAsync(saved).catch(failure);busy=false;controls();}};
$('counting').onclick=()=>{counting=!counting;counter?.missing();$('counting').textContent=counting?'Pausar conteo':'Iniciar conteo';};
$('reset').onclick=()=>{counter?.reset();$('count').textContent='0 reps';};
$('export').onclick=()=>{const url=URL.createObjectURL(new Blob([JSON.stringify({version:1,frames})],{type:'application/json'}));const a=document.createElement('a');a.href=url;a.download='bosu-patron.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);};
$('import').onchange=async()=>{try{const file=$('import').files[0];if(!file)return;if(file.size>500000)throw Error('Archivo demasiado grande.');usePattern(JSON.parse(await file.text()));say('Patrón importado.');}catch(e){failure(e);}};
setInterval(()=>{if(busy||!engine||video.readyState<2)return;$('time').textContent=`${video.currentTime.toFixed(1)} s`;$('seek').value=video.currentTime;if(video.paused||video.ended||video.currentTime===lastTime)return;lastTime=video.currentTime;try{const f=draw();if(counting&&counter){$('count').textContent=`${counter.update(f,performance.now())} reps`;$('phase').textContent=!f?'Cuerpo incompleto: conteo en espera':counter.armed?`Recorrido: ${Math.round(counter.index/(frames.length-1)*100)} %`:'Busca la postura inicial';}}catch(e){counting=false;failure(e);}},100);
window.addEventListener('pagehide',()=>{stopCamera();engine?.close();});
