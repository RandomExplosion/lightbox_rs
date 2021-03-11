import pyttsx3; import gtts                     #TTS
from sys import argv                                #Command line arguments

ttslan = argv[0]                #Get language
ttscontent = argv[1]            #Get Content
filename = argv[2]

tts = gtts.gTTS(ttscontent)     #Init tts engine with the text
tts.lang = ttslan               #Set language
tts.save(filename)              #Save file
