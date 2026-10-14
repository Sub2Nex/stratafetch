// Generate redistribution notices from the exact resolved Cargo package sources.
import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
const metadata=JSON.parse(execFileSync('cargo',['metadata','--format-version','1','--locked','--offline'],{encoding:'utf8',maxBuffer:32*1024*1024}));
const packages=metadata.packages.filter(p=>p.source).sort((a,b)=>`${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`));
let output='StrataFetch third-party dependency notices\nGenerated from Cargo.lock. Dependencies retain their original licenses.\n\n';
let missing=[];
function collect(directory,depth=0){let files=[];for(const entry of fs.readdirSync(directory,{withFileTypes:true})){const full=path.join(directory,entry.name);if(entry.isDirectory()&&depth<2&& !['src','tests','examples','test','benches'].includes(entry.name)){files.push(...collect(full,depth+1));}else if(entry.isFile()&&(/license|copying|copyright|notice|^ofl\.txt$|^ufl\.txt$/i.test(entry.name))&&!/\.(rs|ttf|woff|png|jpg)$/i.test(entry.name)){files.push(full);}}return files;}
for(const pkg of packages){const directory=path.dirname(pkg.manifest_path);output+=`\n${'='.repeat(72)}\n${pkg.name} ${pkg.version}\nDeclared SPDX: ${pkg.license??'license-file'}\nSource: ${pkg.repository??`https://crates.io/crates/${pkg.name}`}\n`;
 const files=collect(directory);if(pkg.license_file&&!files.includes(path.resolve(directory,pkg.license_file))){files.push(path.resolve(directory,pkg.license_file));}
 if(!files.length){missing.push(`${pkg.name}@${pkg.version}`);output+='No separate license text was packaged by this crate. See the source and declared SPDX license.\n';}
 for(const file of files){output+=`\n--- ${path.relative(directory,file).replaceAll('\\','/')} ---\n${fs.readFileSync(file,'utf8').replace(/\r\n?/g,'\n').replace(/[ \t]+$/gm,'')}\n`;}}
fs.writeFileSync('THIRD-PARTY-LICENSES.txt',output);
fs.writeFileSync('docs/DEPENDENCIES.md',`# Dependency notices\n\nThe portable package includes THIRD-PARTY-LICENSES.txt with available license and notice files from ${packages.length} locked dependency packages, including cross-platform and development dependencies. Bundled fonts and trust-root data retain their original licenses.\n\n${missing.length} upstream crates do not ship a separate license text in their published package; their SPDX declaration and source link are recorded:\n\n${missing.map(name=>`- ${name}`).join('\n')}\n\nThis inventory and cargo-deny are engineering checks, not a legal opinion. Review redistribution obligations before a stable release.\n`);
console.log(`Generated notices for ${packages.length} packages; ${missing.length} omit separate license text.`);
