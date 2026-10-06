// Run with: node ref/Notes/verify_examples.js
// Checks the educational algebra and offline UI logic, not engine or browser layout.
'use strict';
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const base = __dirname + '/';
const lib = require(base + 'moments.js');
assert.deepEqual(lib.moments(lib.patterns.centroid), [0,0,2,0,0,0]);
assert.equal(lib.moments(lib.patterns.clustered)[0], -2);
assert.equal(lib.moments(lib.patterns.interdigitated)[0], -1);
assert.equal(lib.moments(lib.patterns.second)[5], 1);
const delta = (cells,a,b) => {
  const mean = label => {
    const v = cells.filter(c=>c.name===label);
    return v.reduce((s,c)=>s+c.x**a*c.y**b,0)/v.length;
  };
  return mean('A')-mean('B');
};
let certificates = 0;
for (const [key,order] of [['centroid',1],['second',2],['third',3]]) {
  const original = lib.cells(lib.patterns[key]);
  for (const cells of [original,original.map(c=>({...c,x:3-c.y,y:-5+c.x}))]) {
    for(let n=1;n<=order;n++) for(let a=0;a<=n;a++) {
      assert.equal(delta(cells,a,n-a),0, `${key}, ${a},${n-a}`);
      certificates++;
    }
  }
}
const broken = lib.cells(lib.patterns.second);
broken[0].x += 1;
assert.notEqual(delta(broken,1,0),0);

// Small DOM harness exercises the actual page scripts without network/browser dependencies.
class Element {
  constructor(tag='div') {this.tagName=tag;this.children=[];this.listeners={};this.attributes={};this.textContent='';this.value='';this.id='';}
  append(...items) {this.children.push(...items);}
  replaceChildren(...items) {this.children=[...items];}
  setAttribute(k,v) {this.attributes[k]=v;}
  addEventListener(k,fn) {this.listeners[k]=fn;}
}
const elements = {};
const make = (id,value='') => {const e=new Element();e.id=id;e.value=value;elements[id]=e;return e;};
for(const id of ['moment-lab','moment-svg','moment-results'])make(id);
make('moment-pattern','centroid');
for(const id of ['gx','gy','qxx','qxy','qyy','c21']) {make(id,id==='qxx'?'1':'0');make(id+'-value');}
const document = {getElementById:id=>elements[id]||null,createElement:tag=>new Element(tag),createElementNS:(_,tag)=>new Element(tag)};
const context=vm.createContext({document});
vm.runInContext(fs.readFileSync(base+'moments.js','utf8'),context);
assert.equal(elements['moment-results'].children.length,5);
assert.equal(elements['moment-svg'].children.filter(x=>x.tagName==='rect').length,4);
elements['moment-pattern'].value='third';elements['moment-pattern'].listeners.change();
assert.equal(elements['moment-svg'].children.filter(x=>x.tagName==='rect').length,16);
elements.qxx.value='0';elements.c21.value='1';elements.c21.listeners.input();
const rows=elements['moment-results'].children;
assert.equal(rows[3].children.at(-1).textContent,'1.000');
assert.equal(rows[4].children.at(-1).textContent,'0.000');
assert.equal(elements['c21-value'].textContent,'1.0');

make('notes-search');make('search-results');
const searchContext=vm.createContext({document,window:{}});
vm.runInContext(fs.readFileSync(base+'search-data.js','utf8'),searchContext);
vm.runInContext(fs.readFileSync(base+'notes.js','utf8'),searchContext);
elements['notes-search'].value='centroid';elements['notes-search'].listeners.input();
assert.ok(elements['search-results'].children.length>1);
assert.ok(elements['search-results'].children.length<=41);
elements['notes-search'].value='quantumUnfindableTerm';elements['notes-search'].listeners.input();
assert.equal(elements['search-results'].children[0].textContent,'0 matching sections');
elements['notes-search'].value='';elements['notes-search'].listeners.input();
assert.equal(elements['search-results'].children.length,0);
console.log(JSON.stringify({passed:true,exact_moment_certificates:certificates,perturbation_detected:true,interactive_pattern_switch:true,slider_recomputation:true,offline_search_positive_empty_and_no_match:true,limitation:'DOM harness; not a browser layout test.'}));
