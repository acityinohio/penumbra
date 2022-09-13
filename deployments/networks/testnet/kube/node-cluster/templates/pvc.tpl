{{ $count := (.Values.count | int) }}
{{ range $i,$e := until $count }}
---
kind: PersistentVolumeClaim
apiVersion: v1
metadata:
  name: "pvc-{{ include "tendermint.name" $ }}-{{$i}}"
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: premium-rwo
  resources:
    requests:
      storage: 100Gi
---
kind: PersistentVolumeClaim
apiVersion: v1
metadata:
  name: "pvc-{{ include "penumbra.name" $ }}-{{$i}}"
spec:
  accessModes:
    - ReadWriteOnce
  storageClassName: premium-rwo
  resources:
    requests:
      storage: 100Gi
{{ end }}
