TODO IN GENERAL:  
	\> Find what media servers actually do and how to implement.  
	\> Commands for server?  
	\> Does the server need to have a topology of the network?  
	\> Check other groups drones and our drone

        **  SERVER:**  
        **CHAT:**  
	 **C \-\> S**: register(\**arguments*\*) (client sends a registration request and server put its id in a hashmap)  
      	 **C \-\> S** : client\_list? (client asks for list of other clients inside the chatroom) 	  
S	 **S \-\> C** : client\_list\!(list\_length, list\_of\_client\_ids) (server responds with list)  
            **C \-\> S** : message\_for?(client\_id, message\_size, message) (client send message to a client using   
s										server)   
            **S \-\> C** : message\_from\!(client\_id, message\_size, message) (server send client a message from   
a										another client)  
            
	S-\>C : error\_client\_not\_registered\! (server responds with this if sending client is not registered   
\[						\[in the hashmap\])

        **MEDIA/TEXT (file and images):**    
	Text server have text files that may have \[image\_id\] inside. Client request text file and then   
           analyze the content, when it encounters an \[image\_id\] it sends request to all media servers for that   
	image file. If media server has that ID it sends the image to the client. To serialize both text file and im	           image file use **Base64 library.** 

*\> Implement media/text server:*  
	**\-C \-\> S :** server\_type? (asks for the type of the server) \[BOTH\]  
**\-S \-\> C** : server\_type\!(type) (server responds with it's type) \[BOTH\]  
	**\-C \-\> S** : files\_list? (asks for the list of text files inside the server) \[TEXT SERVER\]  
	**\-S \-\> C** : files\_list\!(list\_length, list\_of\_file\_ids) (respond with the list of files) \[TEXT SERVER\]  
	**\-S \-\> C** : error\_no\_files\! (respond with this if no files in the server) \[TEXT SERVER\]  
	**\-C \-\> S** : file?(file\_id, list\_length, list\_of\_media\_ids) (asks server for a file) \[TEXT SERVER\]  
	**\-S \-\> C** : file\!(file\_size, file) (responds with the file, converted in binary using github serialize library, and sends the segments to client) \[TEXT SERVER\]  
	**\-S \-\> C** : error\_file\_not\_found\! (responds with this if no file with this id) \[TEXT SERVER\]

	**C \-\> S** : media?(media\_id) (client asks for a media) \[MEDIA SERVER\]  
**S \-\> C** : media\!(media\_size, media) 	(server responds the same as a file) \[MEDIA SERVER\]  
**S \-\> C** : error\_no\_media\! (responds with this if no media inside server) \[MEDIA SERVER\]  
**S \-\> C** : error\_media\_not\_found (responds if no media with that id is found) \[MEDIA SERVER\]

	How do client know which server to contact?  
		We can:  
\-\> After flooding, clients send \[**C \-\> S** : files\_list?\] and to all text servers and save the received lists in memory.  
\-\>Different for images\! In this case clients when it encounters an image id it will ask every server until it finds the one that has it.

