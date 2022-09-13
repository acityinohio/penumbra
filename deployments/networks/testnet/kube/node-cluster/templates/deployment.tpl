{{ $count := (.Values.count | int) }}
{{ range $i,$e := until $count }}
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: "penumbra-{{$i}}"
spec:
  replicas: 1
  selector:
    matchLabels:
      app: "penumbra-{{$i}}"
  template:
    metadata:
      name: "penumbra-{{$i}}"
      labels:
        app: "penumbra-{{$i}}"
        network: "{{ $.Values.network }}"
    spec:
      volumes:
        - name: "pv-{{ include "penumbra.name" $ }}-{{$i}}"
          persistentVolumeClaim:
            claimName: "pvc-{{ include "penumbra.name" $ }}-{{$i}}"
        - name: "pv-{{ include "tendermint.name" $ }}-{{$i}}"
          persistentVolumeClaim:
            claimName: "pvc-{{ include "tendermint.name" $ }}-{{$i}}"
        - name: tm-config
          configMap:
            name: tm-config
            items:
              - key: "config.toml"
                path: "config.toml"
      initContainers:
        - name: heighliner-ownership
          image: busybox
          command:
            - sh
            - -c
            - |
                chown -R 1025:1025 "/home/pv-{{ include "tendermint.name" $ }}-{{$i}}"
                chown -R 1025:1025 "/home/pv-{{ include "penumbra.name" $ }}-{{$i}}"
          volumeMounts:
            - name: "pv-{{ include "tendermint.name" $ }}-{{$i}}"
              mountPath: "/home/pv-{{ include "tendermint.name" $ }}-{{$i}}"
            - name: "pv-{{ include "penumbra.name" $ }}-{{$i}}"
              mountPath: "/home/pv-{{ include "penumbra.name" $ }}-{{$i}}"
        - name: config-init
          image: "{{ $.Values.tendermintImage }}:{{ $.Values.tendermintVersion }}"
          command:
            - sh
            - -c
            - |
              set -eux
              CHAIN_DIR=/home/heighliner/.tendermint
              if [ ! -d $CHAIN_DIR ]; then
                tendermint init full --home $CHAIN_DIR
              else
                TMP_DIR=/home/heighliner/tmpConfig
                tendermint init full --home $TMP_DIR
              fi
          volumeMounts:
            - name: "pv-{{ include "tendermint.name" $ }}-{{$i}}"
              mountPath: /home/heighliner
        - name: config-merge
          image: "{{ $.Values.toolkitImage }}:{{ $.Values.toolkitVersion }}"
          command:
            - sh
            - -c
            - |
              set -eux
              CONFIG_DIR=/home/heighliner/.tendermint/config
              MERGE_DIR=/tmp/configMerge
              OVERLAY_DIR=/config
              TMP_DIR=/home/heighliner/tmpConfig
              if [ -d $TMP_DIR/config ]; then
                mv $TMP_DIR/config/*.toml $CONFIG_DIR/
                rm -rf $TMP_DIR
              fi
              mkdir $MERGE_DIR
              config-merge -f toml $CONFIG_DIR/config.toml $OVERLAY_DIR/config.toml > $MERGE_DIR/config.toml
              dasel put string -f $MERGE_DIR/config.toml -p toml ".p2p.external_address" $(curl -s ifconfig.me):26656
              curl -X GET "http://testnet.penumbra.zone:26657/genesis" -H "accept: application/json" | jq '.result.genesis' > $CONFIG_DIR/genesis.json
              mv $MERGE_DIR/* $CONFIG_DIR/
          securityContext:
            runAsUser: 1025
            runAsGroup: 1025
          volumeMounts:
            - name: "pv-{{ include "tendermint.name" $ }}-{{$i}}"
              mountPath: /home/heighliner
            - name: tm-config
              mountPath: "/config"
              readOnly: true

      containers:
        - name: tm
          image: "{{ $.Values.tendermintImage }}:{{ $.Values.tendermintVersion }}"
          imagePullPolicy: Always
          ports:
            - containerPort: 26657
              protocol: TCP
              name: rpc
            - containerPort: 26656
              protocol: TCP
              name: p2p
          volumeMounts:
            - name: "pv-{{ include "tendermint.name" $ }}-{{$i}}"
              mountPath: /home/heighliner
          command:
            - tendermint
            - start
            - --proxy-app=tcp://localhost:26658
        - name: pd
          image: "{{ $.Values.penumbraImage }}:{{ $.Values.penumbraVersion }}"
          imagePullPolicy: Always
          ports:
            - containerPort: 8080
              protocol: TCP
              name: grpc
            - containerPort: 9000
              protocol: TCP
              name: metrics
          volumeMounts:
            - name: "pv-{{ include "penumbra.name" $ }}-{{$i}}"
              mountPath: /home/heighliner
          command:
            # - sleep
            # - "1000"
            - pd
            - start
            - --home
            - /home/heighliner/pd
        - name: health-check
          image: "{{ $.Values.healthImage }}:{{ $.Values.healthVersion }}"
          imagePullPolicy: IfNotPresent
          ports:
            - containerPort: 1251
              protocol: TCP
              name: health
      dnsPolicy: ClusterFirst
      restartPolicy: Always
      schedulerName: default-scheduler
      terminationGracePeriodSeconds: 30

{{ end }}
