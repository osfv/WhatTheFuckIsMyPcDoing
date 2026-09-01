import { processGroupKey, processLabel } from "./format";
import type { ProcessRow } from "./types";

export interface GroupedProcess {
  key: string;
  name: string;
  count: number;
  cpu: number;
  memoryBytes: number;
  gpu: number;
  gpuMemoryBytes: number;
  diskBps: number;
  cpuPid: number;
  memoryPid: number;
  gpuPid: number;
  diskPid: number;
}

interface Acc extends GroupedProcess {
  maxCpu: number;
  maxMem: number;
  maxGpu: number;
  maxDisk: number;
}

export function groupProcesses(processes: ProcessRow[]): GroupedProcess[] {
  const map = new Map<string, Acc>();
  for (const p of processes) {
    const key = processGroupKey(p.name);
    const gpu = p.gpu ?? 0;
    const gpuMemoryBytes = p.gpuMemoryBytes ?? 0;
    const cur = map.get(key);
    if (!cur) {
      map.set(key, {
        key,
        name: processLabel(p),
        count: 1,
        cpu: p.cpu,
        memoryBytes: p.memoryBytes,
        gpu,
        gpuMemoryBytes,
        diskBps: p.diskBps,
        cpuPid: p.pid,
        memoryPid: p.pid,
        gpuPid: p.pid,
        diskPid: p.pid,
        maxCpu: p.cpu,
        maxMem: p.memoryBytes,
        maxGpu: gpu,
        maxDisk: p.diskBps,
      });
      continue;
    }
    cur.cpu += p.cpu;
    cur.memoryBytes += p.memoryBytes;
    cur.gpu += gpu;
    cur.gpuMemoryBytes += gpuMemoryBytes;
    cur.diskBps += p.diskBps;
    cur.count += 1;
    if (p.cpu > cur.maxCpu) {
      cur.maxCpu = p.cpu;
      cur.cpuPid = p.pid;
    }
    if (p.memoryBytes > cur.maxMem) {
      cur.maxMem = p.memoryBytes;
      cur.memoryPid = p.pid;
    }
    if (gpu > cur.maxGpu) {
      cur.maxGpu = gpu;
      cur.gpuPid = p.pid;
    }
    if (p.diskBps > cur.maxDisk) {
      cur.maxDisk = p.diskBps;
      cur.diskPid = p.pid;
    }
  }
  return [...map.values()].map((acc) => ({
    key: acc.key,
    name: acc.name,
    count: acc.count,
    cpu: acc.cpu,
    memoryBytes: acc.memoryBytes,
    gpu: acc.gpu,
    gpuMemoryBytes: acc.gpuMemoryBytes,
    diskBps: acc.diskBps,
    cpuPid: acc.cpuPid,
    memoryPid: acc.memoryPid,
    gpuPid: acc.gpuPid,
    diskPid: acc.diskPid,
  }));
}
