#set text(lang: "ua")

= Лабораторна робота №2: Distributed lock

Виконав студент групи ФІ-52мн Ісаченко Н. С.

Код лабораторної роботи розміщений на #link("https://github.com/misach2135/distributed_systems_labs/tree/task2/distributed_lock")[github у гілці task2/distributed_lock (цей текст це посилання)].

За основу було взято імплементацію першої лабораторної роботи, дещо реструктуризованої для подальшої роботи: сервіси переміщено в `services/`, rust-крейти в `crates/`, все що повʼязано з інфраструктурою в infrastructure.

У подальшому деякі сервіси, від яких вимагається взаємодія з hazelcast, будуть написані на python з використанням `flask`.

== Розгортання Hazelcast

Hazelcast кластер було розгорнуто в контейнерах, які піднімаються через Docker Compose. Для того щоб підняти hazelcast, треба виконати наступну команду:

```bash
docker compose -f infra/docker-compose.yaml -f infra/task2.docker-compose.override.yaml up
```

Hazelcast кластер описаний в `infra/task2.docker-compose.override.yaml`.

Фрагмент логу, який показує, що кластер складається з трьох вузлів:

```
2026-10-19 20:44:57,124 [ INFO] [main] [c.h.c.LifecycleService]: [172.18.0.6]:5701 [lab] [5.4.0] [172.18.0.6]:5701 is STARTING

2026-10-19 20:44:59,954 [ INFO] [main] [c.h.i.c.i.MulticastJoiner]: [172.18.0.6]:5701 [lab] [5.4.0] Trying to join to discovered node: [172.18.0.7]:5701

2026-10-19 20:44:59,962 [ INFO] [hz.fervent_chandrasekhar.IO.thread-in-2] [c.h.i.s.t.TcpServerConnection]: [172.18.0.6]:5701 [lab] [5.4.0] Initialized new cluster connection between /172.18.0.6:34329 and /172.18.0.7:5701

2026-10-19 20:45:00,081 [ INFO] [hz.fervent_chandrasekhar.IO.thread-in-0] [c.h.i.s.t.TcpServerConnection]: [172.18.0.6]:5701 [lab] [5.4.0] Initialized new cluster connection between /172.18.0.6:5701 and /172.18.0.5:35891

2026-10-19 20:45:00,084 [ INFO] [hz.fervent_chandrasekhar.generic-operation.thread-1] [c.h.i.c.ClusterService]: [172.18.0.6]:5701 [lab] [5.4.0]


Members {size:3, ver:3} [

	Member [172.18.0.7]:5701 - ea4d69d0-4ef3-4533-9da6-313326596097

	Member [172.18.0.5]:5701 - fc42f25c-0151-469d-bee3-8bd5c0da54b4

	Member [172.18.0.6]:5701 - 4986c1b5-b242-4ee8-8ad9-2d0478c971e0 this

]
```

А також, що активовано CP Subsystem:

```
2026-10-19 20:45:03,304 [ INFO] [hz.fervent_chandrasekhar.partition-operation.thread-0] [c.h.c.i.r.i.t.PreVoteTask(METADATA)]: [172.18.0.6]:5701 [lab] [5.4.0] Pre-vote started for next term: 1, last log index: 0, last log term: 0

2026-10-19 20:45:03,305 [ INFO] [hz.fervent_chandrasekhar.partition-operation.thread-0] [c.h.c.i.r.i.RaftNode(METADATA)]: [172.18.0.6]:5701 [lab] [5.4.0]


CP Group Members {groupId: METADATA(0), size:3, term:0, logIndex:0} [

	CPMember{uuid=ea4d69d0-4ef3-4533-9da6-313326596097, address=[172.18.0.7]:5701}

	CPMember{uuid=fc42f25c-0151-469d-bee3-8bd5c0da54b4, address=[172.18.0.5]:5701}

	CPMember{uuid=4986c1b5-b242-4ee8-8ad9-2d0478c971e0, address=[172.18.0.6]:5701} - FOLLOWER this

]

```

== Частина I - Distributed Map

Для демонтрації роботи Distributed Map була реалізована утиліта `utils/hazelcast.py`, яка використовує офіційний hazelecast клієнт. Утиліта дозволяє зробити запис $10000$ значень з ключами від $0$ до $10000$. Та перевірити, скільки фактично ключів збережено, і скільки відсутньо.

Для запису у таблиці потрібно виконати `utils/hazelcast.py fill`. Отримаємо:

#image("assets/hazelcast-after-fill.png")

Після відключення першої ноди, отримаємо такі результати:

#image("assets/hazelcast-after-takeoff-one-node.png")

Дані не втратились, а перерозподілились між іншими вузлами. Після включення вузла, дані знову перерозподілились, і результат не дуже відрізняється від початкового стану:

#image("assets/hazelcast-return-of-first-node.png")

Після відключення послідовно двох нод, частина даних була втрачена, про що дає знати наша утиліта:

#image("assets/two-sequence-node-cli.png")

В menegement center це також помітно:

#image("assets/two-sequence-node-mc.png")

Збільшивши паузу, втрата даниз все одно повторювалась.

Перезапустимо management center, і спробуємо відключити одночасно два вузли.

#image("assets/two-simultaniously-mc.png")

Втрата даних очікувано відбулась.

Для того, щоб уникнути втрат, можна збільшити кількість бекапів на ноду.

```bash
HZ_BACKUP_COUNT=2 docker compose -f infra/docker-compose.yaml -f infra/task2.docker-compose.override.yaml up -d
```

Після відключення двох вузлів посвідовно, всі дані зберіглись.

#image("assets/disable-two-nodes-with-backup.png")

== Частина II: Distributed Lock

Було реалізовано counter-сервіс мовою Python, з використанням офіційного клієнта hazelcast, який використовує FencedLock.

```py
def run(client: hazelcast.HazelcastClient, name: str) -> None:
    lock = client.cp_subsystem.get_lock("counter-lock").blocking()
    iteration = client.cp_subsystem.get_atomic_long("iteration").blocking()
    counter = client.get_map("counter").blocking()

    log.info("%s: waiting for the lock", name)
    fence = lock.lock()
    try:
        i = iteration.get()
        log.info("%s: got the lock (fence=%d), starting from iteration %d", name, fence, i)
        while i < TARGET:
            counter.put("value", i + 1)
            iteration.set(i + 1)
            i += 1
            if i % 1_000 == 0:
                log.info("%s: iteration %d", name, i)
        log.info("%s: done, counter = %d", name, counter.get("value"))
    finally:
        lock.unlock()
```

Цей сервіс, як і інші, запускається через docker compose.

Якщо підняти docker compose, піднімуться 3 сервіси, той що рахує виводить в логах значення каунтера кожні 1000 ітерацій:

#image("assets/active iterator.png")

Інші два каунтера неактивні, і очікують на можливість взяти lock.

#image("assets/really-non-active-lock.png")

По завершенню роботи серсвіса counter-а, відбувається неактивним каунтерам таки вдається перехопити lock, але оскільки counter вже дорахував до 100000, він просто відпускає lock, і завершує свою роботу.

#image("assets/non-active-lock.png")

Якщо під час роботи активного каунтера, вимкнути його, роботу підхватить інший каунтер.

#image("assets/remove-one-counter.png")

#image("assets/another-conter-active.png")

#image("assets/another-conter-still-sleeps.png")


== Висновки:

Distributed Map дозвляє тримати дані між розподіленими вузлами, причому кожен вузол тримає бекап іншого вузла. Гарантія того, що дані не зникнуть, це щоб кожна нода тримала n-1 бекапів вузлів. Тоді навіть якщо одночасно вимкнути два вузла, дані не зникнуть.

Також у лаботаторній роботі було розглянуто роботу Fenced Lock, який дозволяє реалізувати "резервні-воркери", які здатні продовжити виконання задачі, якщо основний воркер впав.
