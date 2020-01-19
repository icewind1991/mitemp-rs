from bluepy import btle

mac = '58:2d:34:35:f3:d4'
p = btle.Peripheral (mac)

for s in p.getServices ():
    print ('Service:', s.uuid)
    for c in s.getCharacteristics ():
        print (' tCharacteristic:', c.uuid)
        print (' t  t', c.propertiesToString ())
        if c.supportsRead ():
            print (' t  t', c.read ())  
