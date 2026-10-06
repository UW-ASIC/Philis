'use strict';
// Original educational example. Equal electrical units and dimensionless pitch;
// these coefficients are illustrative, not process-calibrated predictions.
const PhilisMoments = (() => {
  const patterns = {
    clustered: {name:'Clustered AABB', rows:['AABB']},
    interdigitated: {name:'Interdigitated ABAB', rows:['ABAB']},
    centroid: {name:'Common-centroid ABBA', rows:['ABBA']},
    second: {name:'Second-order: ABBA / BAAB', rows:['ABBA','BAAB']},
    third: {name:'Third-order: ABBA / BAAB / BAAB / ABBA', rows:['ABBA','BAAB','BAAB','ABBA']}
  };
  const powers = [[1,0],[0,1],[2,0],[1,1],[0,2],[2,1]];
  const cells = pattern => pattern.rows.flatMap((row,y) => [...row].map((name,x) => ({name,x:x-(row.length-1)/2,y:(pattern.rows.length-1)/2-y})));
  function moments(pattern) {
    const units = cells(pattern);
    return powers.map(([a,b]) => {
      const mean = name => {const side=units.filter(c=>c.name===name);return side.reduce((s,c)=>s+c.x**a*c.y**b,0)/side.length;};
      return mean('A')-mean('B');
    });
  }
  return {patterns,powers,cells,moments};
})();
if (typeof module !== 'undefined') module.exports = PhilisMoments;
if (typeof document !== 'undefined') (() => {
  const lab = document.getElementById('moment-lab');
  if (!lab) return;
  const selector = document.getElementById('moment-pattern');
  const svg = document.getElementById('moment-svg');
  const tbody = document.getElementById('moment-results');
  const controls = ['gx','gy','qxx','qxy','qyy','c21'].map(id => document.getElementById(id));
  const ns='http://www.w3.org/2000/svg';
  function update() {
    const pattern=PhilisMoments.patterns[selector.value];
    const coefficients=controls.map(c=>Number(c.value));
    controls.forEach(c=>document.getElementById(`${c.id}-value`).textContent=Number(c.value).toFixed(1));
    svg.replaceChildren();
    const title=document.createElementNS(ns,'title');
    title.textContent=pattern.name+'; A units in teal, B units in orange';svg.append(title);
    PhilisMoments.cells(pattern).forEach(c=>{
      const rect=document.createElementNS(ns,'rect');
      rect.setAttribute('x',String(195+c.x*68));rect.setAttribute('y',String(120-c.y*52));
      rect.setAttribute('width','54');rect.setAttribute('height','40');rect.setAttribute('rx','4');
      rect.setAttribute('fill',c.name==='A'?'#00655f':'#98541b');svg.append(rect);
      const label=document.createElementNS(ns,'text');
      label.setAttribute('x',String(222+c.x*68));label.setAttribute('y',String(147-c.y*52));
      label.setAttribute('text-anchor','middle');label.setAttribute('fill','white');label.setAttribute('font-size','22');
      label.textContent=c.name;svg.append(label);
    });
    tbody.replaceChildren();
    Object.entries(PhilisMoments.patterns).forEach(([key,p])=>{
      const delta=PhilisMoments.moments(p);const tr=document.createElement('tr');
      const title=document.createElement('th');title.scope='row';title.textContent=p.name+(key===selector.value?' (shown)':'');tr.append(title);
      [...delta,delta.reduce((s,x,i)=>s+x*coefficients[i],0)].forEach(v=>{const td=document.createElement('td');td.textContent=v.toFixed(3);tr.append(td);});
      tbody.append(tr);
    });
  }
  selector.addEventListener('change',update);controls.forEach(c=>c.addEventListener('input',update));update();
})();
